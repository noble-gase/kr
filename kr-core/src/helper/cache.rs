use std::{future::Future, time::Duration};

use redis::{AsyncCommands, RedisResult};
use serde::{de::DeserializeOwned, Serialize};

use crate::redix;

pub const HSET: &str = r#"
redis.call('HSET', KEYS[1], ARGV[1], ARGV[2])
if redis.call('TTL', KEYS[1]) == -1 then
    redis.call('EXPIRE', KEYS[1], ARGV[3])
end
"#;

pub async fn get_or_set<T, F, Fut>(
    pool: redix::Pool,
    key: impl AsRef<str>,
    loader: F,
    ttl: Option<Duration>,
) -> anyhow::Result<Option<T>>
where
    T: Serialize + DeserializeOwned + Send + 'static,
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<Option<T>>>,
{
    match pool {
        redix::Pool::Single(p) => {
            let mut conn = p.get().await?;

            let key = key.as_ref();

            // 从缓存读取
            let ret_get: Option<String> = conn.get(key).await?;
            if let Some(v) = ret_get {
                let parsed = serde_json::from_str(&v)?;
                return Ok(parsed);
            }

            // 缓存未命中，调用loader获取数据
            let data = loader().await?;

            // 数据存在，写入缓存
            if let Some(v) = &data {
                let json_str = serde_json::to_string(&v)?;
                let set_ret: RedisResult<()> = match ttl {
                    Some(d) => conn.set_ex(key, &json_str, d.as_secs()).await,
                    None => conn.set(key, &json_str).await,
                };
                if let Err(e) = set_ret {
                    tracing::error!(error = ?e, key = key, data = json_str, "[cache::get_or_set] set data failed")
                }
            }

            Ok(data)
        }
        redix::Pool::Cluster(p) => {
            let mut conn = p.get().await?;

            let key = key.as_ref();

            // 从缓存读取
            let ret_get: Option<String> = conn.get(key).await?;
            if let Some(v) = ret_get {
                let parsed = serde_json::from_str(&v)?;
                return Ok(parsed);
            }

            // 缓存未命中，调用loader获取数据
            let data = loader().await?;

            // 数据存在，写入缓存
            if let Some(v) = &data {
                let json_str = serde_json::to_string(&v)?;
                let set_ret: RedisResult<()> = match ttl {
                    Some(d) => conn.set_ex(key, &json_str, d.as_secs()).await,
                    None => conn.set(key, &json_str).await,
                };
                if let Err(e) = set_ret {
                    tracing::error!(error = ?e, key = key, data = json_str, "[cache::get_or_set] set data failed")
                }
            }

            Ok(data)
        }
    }
}

pub async fn hget_or_hset<T, F, Fut>(
    pool: redix::Pool,
    key: impl AsRef<str>,
    field: impl AsRef<str>,
    loader: F,
    ttl: Option<Duration>,
) -> anyhow::Result<Option<T>>
where
    T: Serialize + DeserializeOwned + Send + 'static,
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<Option<T>>>,
{
    match pool {
        redix::Pool::Single(p) => {
            let mut conn = p.get().await?;

            let key = key.as_ref();
            let field = field.as_ref();

            // 从缓存读取
            let ret_get: Option<String> = conn.hget(key, field).await?;
            if let Some(v) = ret_get {
                let parsed = serde_json::from_str(&v)?;
                return Ok(parsed);
            }

            // 缓存未命中，调用loader获取数据
            let data = loader().await?;

            // 数据存在，写入缓存
            if let Some(v) = &data {
                let json_str = serde_json::to_string(&v)?;
                let set_ret: RedisResult<()> = match ttl {
                    Some(d) => {
                        redis::Script::new(HSET)
                            .key(key)
                            .arg(field)
                            .arg(&json_str)
                            .arg(d.as_secs() as i64)
                            .invoke_async(&mut *conn)
                            .await
                    }
                    None => conn.hset(key, field, &json_str).await,
                };
                if let Err(e) = set_ret {
                    tracing::error!(error = ?e, key = key, data = json_str, "[cache::hget_or_hset] set data failed")
                }
            }

            Ok(data)
        }
        redix::Pool::Cluster(p) => {
            let mut conn = p.get().await?;

            let key = key.as_ref();
            let field = field.as_ref();

            // 从缓存读取
            let ret_get: Option<String> = conn.hget(key, field).await?;
            if let Some(v) = ret_get {
                let parsed = serde_json::from_str(&v)?;
                return Ok(parsed);
            }

            // 缓存未命中，调用loader获取数据
            let data = loader().await?;

            // 数据存在，写入缓存
            if let Some(v) = &data {
                let json_str = serde_json::to_string(&v)?;
                let set_ret: RedisResult<()> = match ttl {
                    Some(d) => {
                        redis::Script::new(HSET)
                            .key(key)
                            .arg(field)
                            .arg(&json_str)
                            .arg(d.as_secs() as i64)
                            .invoke_async(&mut *conn)
                            .await
                    }
                    None => conn.hset(key, field, &json_str).await,
                };
                if let Err(e) = set_ret {
                    tracing::error!(error = ?e, key = key, data = json_str, "[cache::hget_or_hset] set data failed")
                }
            }

            Ok(data)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use anyhow::Ok;
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize, Serialize)]
    struct Demo {
        id: i64,
        name: String,
    }

    #[tokio::test]
    async fn test_get_or_set() {
        let pool = redix::open::<redix::Single>(vec!["redis://127.0.0.1:6379".to_string()], None)
            .await
            .unwrap();

        let ret = get_or_set(
            redix::Pool::Single(pool),
            "test_get_or_set",
            || async {
                println!("call loader");
                Ok(Some(Demo {
                    id: 1,
                    name: "hello".to_string(),
                }))
            },
            Some(Duration::from_mins(5)),
        )
        .await
        .unwrap();

        println!("{:#?}", ret);
    }

    #[tokio::test]
    async fn test_hget_or_hset() {
        let pool = redix::open::<redix::Single>(vec!["redis://127.0.0.1:6379".to_string()], None)
            .await
            .unwrap();

        let ret = hget_or_hset(
            redix::Pool::Single(pool),
            "test_hget_or_hset",
            "hello",
            || async {
                println!("call loader");
                Ok(Some(Demo {
                    id: 1,
                    name: "hello".to_string(),
                }))
            },
            Some(Duration::from_mins(5)),
        )
        .await
        .unwrap();

        println!("{:#?}", ret);
    }
}
