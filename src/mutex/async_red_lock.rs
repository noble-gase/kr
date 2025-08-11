use bon::bon;
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
/// let mut lock = AsyncRedLock::acquire().pool(pool).key("key").ttl(Duration::from_secs(10)).call().await?;
/// if lock.is_none() {
///     return Err("operation is too frequent, please try again later")
/// }
/// // do something ...
/// // 释放锁
/// lock.unwrap().release().await?;
///
/// // 尝试获取锁（重试3次，间隔100ms）
/// let mut lock = AsyncRedLock::acquire().pool(pool).key("key").ttl(Duration::from_secs(10)).retry((3, Duration::from_millis(100))).call().await?;
/// if lock.is_none() {
///     return Err("operation is too frequent, please try again later")
/// }
/// // do something ...
/// // 释放锁
/// lock.unwrap().release().await?;
/// ```
pub struct AsyncRedLock<'a> {
    pool: &'a bb8::Pool<bb8_redis::RedisConnectionManager>,
    key: String,
    ttl: u64,
    token: Option<String>,
    prevent: bool,
}

#[bon]
impl<'a> AsyncRedLock<'a> {
    /// 获取锁
    #[builder]
    pub async fn acquire(
        pool: &'a bb8::Pool<bb8_redis::RedisConnectionManager>,
        key: &str,
        ttl: time::Duration,
        retry: Option<(i32, time::Duration)>,
    ) -> anyhow::Result<Option<Self>> {
        let mut red_lock = AsyncRedLock {
            pool,
            key: key.to_string(),
            ttl: ttl.as_millis() as u64,
            token: None,
            prevent: false,
        };

        if let Some((attempts, interval)) = retry {
            let threshold = attempts - 1;
            for i in 0..attempts {
                red_lock.set_nx().await?;
                if red_lock.token.is_some() {
                    return Ok(Some(red_lock));
                }
                if i < threshold {
                    sleep(interval).await;
                }
            }
            return Ok(None);
        }

        red_lock.set_nx().await?;
        if red_lock.token.is_none() {
            return Ok(None);
        }
        Ok(Some(red_lock))
    }

    /// 手动释放锁
    pub async fn release(&mut self) -> anyhow::Result<()> {
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

    /// 阻止 `AsyncDrop` 自动释放锁
    pub fn prevent(&mut self) {
        self.prevent = true;
    }

    async fn set_nx(&mut self) -> anyhow::Result<()> {
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
// impl AsyncDrop for AsyncRedLock<'_> {
//     fn drop(&mut self) {
//         if self.prevent || self.token.is_none() {
//             return;
//         }

//         // 释放锁
//         let ret = self.release().await;
//         if let Err(e) = ret {
//             tracing::error!(err = ?e, "[mutex.async_red_lock] drop release(key={}) failed", self.key);
//         }
//     }
// }
