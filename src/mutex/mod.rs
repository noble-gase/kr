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
/// // 获取锁
/// let mut mutex = mutex::RedLock::lock(pool, "key", Duration::from_secs(10), None)?;
/// if mutex.is_none() {
///     return Err("Operation is too frequent, please try again later")
/// }
/// let mut mutex = mutex.unwrap();
/// // do something
/// mutex.unlock()?;
///
/// // 尝试获取锁（重试3次，间隔100ms）
/// let mut mutex = mutex::RedLock::lock(pool, "key", Duration::from_secs(10), Some((3, Duration::from_millis(100))))?;
/// if mutex.is_none() {
///     return Err("Operation is too frequent, please try again later")
/// }
/// let mut mutex = mutex.unwrap();
/// // do something
/// mutex.unlock()?;
/// ```
pub struct RedLock<'a> {
    pool: &'a r2d2::Pool<redis::Client>,
    key: String,
    ttl: u64,
    token: Option<String>,
}

impl<'a> RedLock<'a> {
    /// 获取锁
    pub fn lock(
        client: &'a r2d2::Pool<redis::Client>,
        key: &str,
        ttl: time::Duration,
        retry: Option<(i32, time::Duration)>,
    ) -> anyhow::Result<Option<Self>> {
        let mut red_lock = RedLock {
            pool: client,
            key: key.to_string(),
            ttl: ttl.as_millis() as u64,
            token: None,
        };

        // 重试模式
        if let Some((attempts, interval)) = retry {
            for i in 0..attempts {
                red_lock._acquire()?;
                if red_lock.token.is_some() {
                    return Ok(Some(red_lock));
                }
                if i < attempts - 1 {
                    thread::sleep(interval);
                }
            }
            return Ok(None);
        }

        // 一次性模式
        red_lock._acquire()?;
        if red_lock.token.is_none() {
            return Ok(None);
        }
        Ok(Some(red_lock))
    }

    /// 手动释放锁
    pub fn unlock(&mut self) -> anyhow::Result<()> {
        if self.token.is_none() {
            return Ok(());
        }
        let mut conn = self.pool.get()?;
        let script = redis::Script::new(SCRIPT);
        script
            .key(&self.key)
            .arg(&self.token)
            .invoke::<()>(&mut conn)?;
        self.token = None;
        Ok(())
    }

    fn _acquire(&mut self) -> anyhow::Result<()> {
        let mut conn = self.pool.get()?;
        let opts = redis::SetOptions::default()
            .conditional_set(NX)
            .with_expiration(PX(self.ttl));
        let token = Uuid::new_v4().to_string();

        let ret_setnx: redis::RedisResult<bool> = conn.set_options(&self.key, &token, opts);
        match ret_setnx {
            Ok(v) => {
                if v {
                    self.token = Some(token);
                }
                Ok(())
            }
            Err(e) => {
                // 尝试GET一次：避免因redis网络错误导致误加锁
                let ret_get: Option<String> = conn.get(&self.key)?;
                let v = ret_get.ok_or(e)?;
                if v == token {
                    self.token = Some(token);
                }
                Ok(())
            }
        }
    }
}

/// 自动释放锁
impl Drop for RedLock<'_> {
    fn drop(&mut self) {
        if self.token.is_none() {
            return;
        }

        // 释放锁
        let ret = self.unlock();
        if let Err(e) = ret {
            tracing::error!(err = ?e, "[mutex.red_lock] drop unlock(key={}) failed", self.key);
        }
    }
}

/// 基于Redis的异步分布式锁
/// # Examples
///
/// ```no_run
/// // 获取锁
/// let mut mutex = mutex::RedAsyncLock::lock(pool, "key", 10, None).await?;
/// if mutex.is_none() {
///     return Err("Operation is too frequent, please try again later")
/// }
/// let mut mutex = mutex.unwrap();
/// // do something
/// mutex.unlock().await?;
///
/// // 尝试获取锁（重试3次，间隔100ms）
/// let mut mutex = mutex::RedAsyncLock::lock(pool, "key", 10, Some((3, Duration::from_millis(100)))).await?;
/// if mutex.is_none() {
///     return Err("Operation is too frequent, please try again later")
/// }
/// let mut mutex = mutex.unwrap();
/// // do something
/// mutex.unlock().await?;
/// ```
pub struct RedAsyncLock<'a> {
    pool: &'a bb8::Pool<async_redis::AsyncConnManager>,
    key: String,
    ttl: u64,
    token: Option<String>,
}

impl<'a> RedAsyncLock<'a> {
    /// 获取锁
    pub async fn lock(
        pool: &'a bb8::Pool<async_redis::AsyncConnManager>,
        key: &str,
        ttl: time::Duration,
        retry: Option<(i32, time::Duration)>,
    ) -> anyhow::Result<Option<Self>> {
        let mut red_lock = RedAsyncLock {
            pool,
            key: key.to_string(),
            ttl: ttl.as_millis() as u64,
            token: None,
        };

        if let Some((attempts, interval)) = retry {
            for i in 0..attempts {
                red_lock._acquire().await?;
                if red_lock.token.is_some() {
                    return Ok(Some(red_lock));
                }
                if i < attempts - 1 {
                    sleep(interval).await;
                }
            }
            return Ok(None);
        }

        red_lock._acquire().await?;
        if red_lock.token.is_none() {
            return Ok(None);
        }
        Ok(Some(red_lock))
    }

    /// 手动释放锁
    pub async fn unlock(&mut self) -> anyhow::Result<()> {
        if self.token.is_none() {
            return Ok(());
        }
        let mut conn = self.pool.get().await?;
        let script = redis::Script::new(SCRIPT);
        script
            .key(&self.key)
            .arg(&self.token)
            .invoke_async::<()>(&mut *conn)
            .await?;
        self.token = None;
        Ok(())
    }

    async fn _acquire(&mut self) -> anyhow::Result<()> {
        let mut conn = self.pool.get().await?;
        let opts = redis::SetOptions::default()
            .conditional_set(NX)
            .with_expiration(PX(self.ttl));
        let token = Uuid::new_v4().to_string();

        let ret_setnx: redis::RedisResult<bool> = conn.set_options(&self.key, &token, opts).await;
        match ret_setnx {
            Ok(v) => {
                if v {
                    self.token = Some(token);
                }
                Ok(())
            }
            Err(e) => {
                // 尝试GET一次：避免因redis网络错误导致误加锁
                let ret_get: Option<String> = conn.get(&self.key).await?;
                let v = ret_get.ok_or(e)?;
                if v == token {
                    self.token = Some(token);
                }
                Ok(())
            }
        }
    }
}

/// 自动释放锁
/// TODO: AsyncDrop
impl Drop for RedAsyncLock<'_> {
    fn drop(&mut self) {
        if self.token.is_none() {
            return;
        }

        let pool = self.pool.clone();
        let key = self.key.clone();
        let token = self.token.clone();

        // 异步释放锁
        tokio::spawn(async move {
            if let Ok(mut conn) = pool.get().await {
                let script = redis::Script::new(SCRIPT);
                let ret = script
                    .key(&key)
                    .arg(&token)
                    .invoke_async::<()>(&mut *conn)
                    .await;
                if let Err(e) = ret {
                    tracing::error!(err = ?e, "[mutex.red_async_lock] drop unlock(key={}) failed", key);
                }
            }
        });
    }
}
