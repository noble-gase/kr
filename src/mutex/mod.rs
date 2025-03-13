use std::{thread, time};

use redis::{AsyncCommands, Commands, ExistenceCheck::NX, SetExpiry::PX};
use tokio::time::sleep;
use uuid::Uuid;

use crate::manager::async_redis;

pub const SCRIPT: &str = r#"
if redis.call('get', KEYS[1]) == ARGV[1] then
    return redis.call('del', KEYS[1])
else
    return 0
end
"#;

/// 基于Redis的分布式锁
/// # Examples
///
/// ```no_run
/// let mut mutex = mutex::RedLock::new((pool, async_pool), "key".to_string(), Duration::from_secs(60), true);
/// let ok = mutex.async_lock().await?;
/// if !ok  {
///     return Err(Code::ErrFrequent(None))
/// }
/// ```
pub struct RedLock<'a> {
    pool: &'a r2d2::Pool<redis::Client>,
    async_pool: &'a bb8::Pool<async_redis::AsyncConnManager>,
    key: String,
    token: String,
    expire: u64,
    unlock: bool,
}

impl<'a> RedLock<'a> {
    pub fn new(
        client: (
            &'a r2d2::Pool<redis::Client>,
            &'a bb8::Pool<async_redis::AsyncConnManager>,
        ),
        key: String,
        ttl: time::Duration,
        auto_unlock: bool,
    ) -> RedLock<'a> {
        let (pool, async_pool) = client;
        RedLock {
            pool,
            async_pool,
            key,
            token: String::from(""),
            expire: ttl.as_millis() as u64,
            unlock: auto_unlock,
        }
    }

    /// 获取锁（同步）
    pub fn lock(&mut self) -> anyhow::Result<bool> {
        self._acquire()
    }
    /// 获取锁（异步）
    pub async fn async_lock(&mut self) -> anyhow::Result<bool> {
        self._async_acquire().await
    }

    /// 尝试获取锁（同步）
    pub fn try_lock(&mut self, attempts: i32, interval: time::Duration) -> anyhow::Result<bool> {
        for i in 0..attempts {
            let ok = self._acquire()?;
            if ok {
                return Ok(true);
            }
            if i < attempts - 1 {
                thread::sleep(interval);
            }
        }
        Ok(false)
    }
    /// 尝试获取锁（异步）
    pub async fn async_try_lock(
        &mut self,
        attempts: i32,
        interval: time::Duration,
    ) -> anyhow::Result<bool> {
        for i in 0..attempts {
            let ok = self._async_acquire().await?;
            if ok {
                return Ok(true);
            }
            if i < attempts - 1 {
                sleep(interval).await;
            }
        }
        Ok(false)
    }

    /// 手动释放锁（同步）
    pub fn unlock(&mut self) -> anyhow::Result<()> {
        if self.token.is_empty() {
            return Ok(());
        }
        let mut conn = self.pool.get()?;
        let script = redis::Script::new(SCRIPT);
        script
            .key(&self.key)
            .arg(&self.token)
            .invoke::<()>(&mut *conn)?;
        Ok(())
    }
    /// 手动释放锁（异步）
    pub async fn async_unlock(&mut self) -> anyhow::Result<()> {
        if self.token.is_empty() {
            return Ok(());
        }
        let mut conn = self.async_pool.get().await?;
        let script = redis::Script::new(SCRIPT);
        script
            .key(&self.key)
            .arg(&self.token)
            .invoke_async::<()>(&mut *conn)
            .await?;
        Ok(())
    }

    fn _acquire(&mut self) -> anyhow::Result<bool> {
        let mut conn = self.pool.get()?;
        let opts = redis::SetOptions::default()
            .conditional_set(NX)
            .with_expiration(PX(self.expire));
        let token = Uuid::new_v4().to_string();

        let ret_setnx: redis::RedisResult<bool> = conn.set_options(&self.key, &token, opts);
        match ret_setnx {
            Ok(v) => {
                if v {
                    self.token = token;
                    return Ok(true);
                }
                Ok(false)
            }
            Err(e) => {
                // 尝试GET一次：避免因redis网络错误导致误加锁
                let ret_get: Option<String> = conn.get(&self.key)?;
                let v = ret_get.ok_or(e)?;
                if v == token {
                    self.token = token;
                    return Ok(true);
                }
                Ok(false)
            }
        }
    }
    async fn _async_acquire(&mut self) -> anyhow::Result<bool> {
        let mut conn = self.async_pool.get().await?;
        let opts = redis::SetOptions::default()
            .conditional_set(NX)
            .with_expiration(PX(self.expire));
        let token = Uuid::new_v4().to_string();

        let ret_setnx: redis::RedisResult<bool> = conn.set_options(&self.key, &token, opts).await;
        match ret_setnx {
            Ok(v) => {
                if v {
                    self.token = token;
                    return Ok(true);
                }
                Ok(false)
            }
            Err(e) => {
                // 尝试GET一次：避免因redis网络错误导致误加锁
                let ret_get: Option<String> = conn.get(&self.key).await?;
                let v = ret_get.ok_or(e)?;
                if v == token {
                    self.token = token;
                    return Ok(true);
                }
                Ok(false)
            }
        }
    }
}

/// 自动释放锁
impl Drop for RedLock<'_> {
    fn drop(&mut self) {
        if !self.unlock || self.token.is_empty() {
            return;
        }

        let mut conn = match self.pool.get() {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(err = ?e, "[mutex.red_lock] redis get connection error");
                return;
            }
        };

        let script = redis::Script::new(SCRIPT);
        let ret: redis::RedisResult<()> = script.key(&self.key).arg(&self.token).invoke(&mut conn);
        if let Err(e) = ret {
            tracing::error!(err = ?e, "[mutex.red_lock] redis del key({}) error", self.key);
        }
    }
}
