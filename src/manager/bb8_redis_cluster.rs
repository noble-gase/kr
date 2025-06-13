use redis::{cluster, cluster_async};

#[derive(Clone)]
pub struct RedisConnectionManager {
    client: cluster::ClusterClient,
}

impl RedisConnectionManager {
    pub fn new(c: cluster::ClusterClient) -> Self {
        Self { client: c }
    }
}

/// 异步集群连接管理器
///
/// # Example
///
/// ```
/// let manager = bb8_redis_cluster::RedisConnectionManager::new(redis::cluster::ClusterClient::new(vec!["redis://127.0.0.1:6379"]).unwrap());
/// ```
impl bb8::ManageConnection for RedisConnectionManager {
    type Connection = cluster_async::ClusterConnection;
    type Error = redis::RedisError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let c = self.client.get_async_connection().await?;
        Ok(c)
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        let pong: String = redis::cmd("PING").query_async(conn).await?;
        match pong.as_str() {
            "PONG" => Ok(()),
            _ => Err((redis::ErrorKind::ResponseError, "ping request").into()),
        }
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}
