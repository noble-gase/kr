use bon::bon;
use redis::{Commands, ExistenceCheck::NX, SetExpiry::PX};
use std::{thread, time};
use uuid::Uuid;

/// 基于Redis的分布式锁
///
/// # Examples
///
/// ```
/// // 获取锁
/// let mut lock = RedLock::acquire().pool(pool).key("key").ttl(Duration::from_secs(10)).call()?;
/// if lock.is_none() {
///     return Err("operation is too frequent, please try again later")
/// }
/// // do something ...
/// // 释放锁
/// lock.unwrap().release()?;
///
/// // 尝试获取锁（重试3次，间隔100ms）
/// let mut lock = RedLock::acquire().pool(pool).key("key").ttl(Duration::from_secs(10)).retry((3, Duration::from_millis(100))).call()?;
/// if lock.is_none() {
///     return Err("operation is too frequent, please try again later")
/// }
/// // do something ...
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

#[bon]
impl<'a> RedLock<'a> {
    /// 获取锁
    #[builder]
    pub fn acquire(
        pool: &'a r2d2::Pool<redis::Client>,
        #[builder(into)] key: String,
        ttl: time::Duration,
        retry: Option<(i32, time::Duration)>,
    ) -> anyhow::Result<Option<Self>> {
        let mut red_lock = RedLock {
            pool,
            key,
            ttl: ttl.as_millis() as u64,
            token: None,
            prevent: false,
        };

        // 重试模式
        if let Some((attempts, duration)) = retry {
            let threshold = attempts - 1;
            for i in 0..attempts {
                red_lock.set_nx()?;
                if red_lock.token.is_some() {
                    return Ok(Some(red_lock));
                }
                if i < threshold {
                    thread::sleep(duration);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_red_lock() {
        let pool = r2d2::Pool::new(redis::Client::open("redis://127.0.0.1:6379").unwrap()).unwrap();
        let lock = RedLock::acquire()
            .pool(&pool)
            .key("test")
            .ttl(time::Duration::from_secs(10))
            .call()
            .unwrap();
        assert!(lock.is_some());
    }
}
