use redis::{AsyncCommands, ExistenceCheck::NX, SetExpiry::PX};
use std::time;
use tokio::time::sleep;
use uuid::Uuid;

use crate::manager::bb8_redis;

/// 基于Redis的异步分布式锁
///
/// # Examples
///
/// ```
/// // 获取锁
/// let mut lock = RedLock::lock(pool, "key", 10, None).await?;
/// if lock.is_none() {
///     return Err("Operation is too frequent, please try again later")
/// }
/// // do something
/// // 释放锁
/// lock.unwrap().unlock().await?;
///
/// // 尝试获取锁（重试3次，间隔100ms）
/// let mut lock = RedLock::lock(pool, "key", 10, Some((3, Duration::from_millis(100)))).await?;
/// if lock.is_none() {
///     return Err("Operation is too frequent, please try again later")
/// }
/// // do something
/// // 释放锁
/// lock.unwrap().unlock().await?;
/// ```
pub struct RedLock<'a> {
    pool: &'a bb8::Pool<bb8_redis::RedisConnectionManager>,
    key: String,
    ttl: u64,
    token: Option<String>,
}

impl<'a> RedLock<'a> {
    /// 获取锁
    pub async fn lock(
        pool: &'a bb8::Pool<bb8_redis::RedisConnectionManager>,
        key: &str,
        ttl: time::Duration,
        retry: Option<(i32, time::Duration)>,
    ) -> anyhow::Result<Option<Self>> {
        let mut red_lock = RedLock {
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
        let script = redis::Script::new(super::SCRIPT);
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

// 自动释放锁
// TODO: AsyncDrop
// impl AsyncDrop for RedLock<'_> {
//     fn drop(&mut self) {
//         if self.token.is_none() {
//             return;
//         }

//         // 释放锁
//         let ret = self.unlock().await;
//         if let Err(e) = ret {
//             tracing::error!(err = ?e, "[mutex.red_async_lock] drop unlock(key={}) failed", self.key);
//         }
//     }
// }
