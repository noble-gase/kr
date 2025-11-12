use bon::bon;
use redis::{AsyncCommands, ExistenceCheck::NX, SetExpiry::PX};
use std::time;
use tokio::time::sleep;
use uuid::Uuid;

use crate::manager::bb8_redis;

/// 基于Redis的异步分布式锁（离开作用域自动释放）
///
/// # Examples
///
/// ```
/// // 获取锁
/// let mut lock = AsyncRedLock::acquire()
///     .pool(pool.clone())
///     .key("key")
///     .ttl(Duration::from_secs(10))
///     .call()
///     .await?;
/// if lock.is_none() {
///     return Err("operation is too frequent, please try again later")
/// }
/// // 手动释放
/// lock.unwrap().release().await?;
///
/// // 尝试获取锁（重试3次，间隔100ms）
/// let mut lock = AsyncRedLock::acquire()
///     .pool(pool.clone())
///     .key("key")
///     .ttl(Duration::from_secs(10))
///     .retry((3, Duration::from_millis(100)))
///     .call()
///     .await?;
/// if lock.is_none() {
///     return Err("operation is too frequent, please try again later")
/// }
/// // 手动释放
/// lock.unwrap().release().await?;
/// ```
pub struct AsyncRedLock {
    pool: bb8::Pool<bb8_redis::RedisConnectionManager>,
    key: String,
    ttl: time::Duration,
    token: Option<String>,
    prevent: bool,
}

#[bon]
impl AsyncRedLock {
    /// 获取锁
    #[builder]
    pub async fn acquire(
        pool: bb8::Pool<bb8_redis::RedisConnectionManager>,
        #[builder(into)] key: String,
        ttl: time::Duration,
        retry: Option<(i32, time::Duration)>,
    ) -> anyhow::Result<Option<Self>> {
        let mut red_lock = AsyncRedLock {
            pool,
            key,
            ttl,
            token: None,
            prevent: false,
        };

        if let Some((attempts, duration)) = retry {
            let threshold = attempts - 1;
            for i in 0..attempts {
                red_lock.set_nx().await?;
                if red_lock.token.is_some() {
                    return Ok(Some(red_lock));
                }
                if i < threshold {
                    sleep(duration).await;
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
            .with_expiration(PX(self.ttl.as_millis() as u64));

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
impl Drop for AsyncRedLock {
    fn drop(&mut self) {
        if self.prevent || self.token.is_none() {
            return;
        }

        let pool = self.pool.clone();
        let key = self.key.clone();
        let token = self.token.clone().unwrap();

        // 异步释放锁
        tokio::spawn(async move {
            if let Err(e) = async {
                let mut conn = pool.get().await?;
                let script = redis::Script::new(super::SCRIPT);
                script
                    .key(&key)
                    .arg(&token)
                    .invoke_async::<()>(&mut *conn)
                    .await?;
                Ok::<_, anyhow::Error>(())
            }
            .await
            {
                tracing::error!(err = ?e, "[mutex.async_red_lock] drop release(key={}) failed", key);
            }
        });
    }
}

// 自动释放锁
// impl AsyncDrop for AsyncRedLock {
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::manager::bb8_redis::RedisConnectionManager;

    use super::*;

    #[tokio::test]
    async fn test_async_red_lock() {
        let pool = bb8::Pool::builder()
            .build(RedisConnectionManager::new(
                redis::Client::open("redis://127.0.0.1:6379").unwrap(),
            ))
            .await
            .unwrap();
        {
            let lock = AsyncRedLock::acquire()
                .pool(pool)
                .key("test")
                .ttl(time::Duration::from_secs(10))
                .call()
                .await
                .unwrap();
            assert!(lock.is_some());
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
