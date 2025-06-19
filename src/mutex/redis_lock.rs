use redis::{Commands, ExistenceCheck::NX, SetExpiry::PX};
use std::{thread, time};
use uuid::Uuid;

/// 基于Redis的分布式锁
///
/// # Examples
///
/// ```
/// // 获取锁
/// let mut lock = RedLock::acquire(pool, "key", Duration::from_secs(10), None)?;
/// if lock.is_none() {
///     return Err("operation is too frequent, please try again later")
/// }
/// // do something
/// // 释放锁
/// lock.unwrap().release()?;
///
/// // 尝试获取锁（重试3次，间隔100ms）
/// let mut lock = RedLock::acquire(pool, "key", Duration::from_secs(10), Some((3, Duration::from_millis(100))))?;
/// if lock.is_none() {
///     return Err("operation is too frequent, please try again later")
/// }
/// // do something
/// // 释放锁
/// lock.unwrap().release()?;
/// ```
pub struct RedLock<'a> {
    pool: &'a r2d2::Pool<redis::Client>,
    key: String,
    ttl: u64,
    token: Option<String>,
    prevent: bool,
}

impl<'a> RedLock<'a> {
    /// 获取锁
    pub fn acquire(
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
            prevent: false,
        };

        // 重试模式
        if let Some((attempts, interval)) = retry {
            for i in 0..attempts {
                red_lock.set_nx()?;
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
        red_lock.set_nx()?;
        if red_lock.token.is_none() {
            return Ok(None);
        }
        Ok(Some(red_lock))
    }

    /// 手动释放锁
    pub fn release(&mut self) -> anyhow::Result<()> {
        if self.token.is_none() {
            return Ok(());
        }

        let mut conn = self.pool.get()?;
        let script = redis::Script::new(super::SCRIPT);
        script
            .key(&self.key)
            .arg(&self.token)
            .invoke::<()>(&mut conn)?;
        self.token = None;
        Ok(())
    }

    /// 阻止 `Drop` 自动释放锁
    pub fn prevent(&mut self) {
        self.prevent = true;
    }

    fn set_nx(&mut self) -> anyhow::Result<()> {
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
        if self.prevent || self.token.is_none() {
            return;
        }

        // 释放锁
        let ret = self.release();
        if let Err(e) = ret {
            tracing::error!(err = ?e, "[mutex.red_lock] drop release(key={}) failed", self.key);
        }
    }
}
