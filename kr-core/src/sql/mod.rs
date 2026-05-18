pub mod mysql;
pub mod pgsql;
pub mod sqlite;

use std::{sync::OnceLock, time::Duration};

use sqlx::{
    mysql::MySqlPoolOptions, pool::PoolOptions, postgres::PgPoolOptions, sqlite::SqlitePoolOptions, Database, MySql, Pool, Postgres, Sqlite,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertOutcome<T> {
    Inserted(T),
    Duplicate,
}

#[inline]
pub fn is_unique_violation(err: &sqlx::Error) -> bool {
    err.as_database_error().is_some_and(|db| db.is_unique_violation())
}

#[inline]
pub fn is_unique_violation_anyhow(err: &anyhow::Error) -> bool {
    err.downcast_ref::<sqlx::Error>().is_some_and(is_unique_violation)
}

pub trait Factory {
    type DB: Database;

    fn build() -> PoolOptions<Self::DB>;
}

pub struct MySQL;

impl Factory for MySQL {
    type DB = MySql;

    fn build() -> PoolOptions<Self::DB> {
        MySqlPoolOptions::new()
    }
}

pub struct PgSQL;

impl Factory for PgSQL {
    type DB = Postgres;

    fn build() -> PoolOptions<Self::DB> {
        PgPoolOptions::new()
    }
}

pub struct SQLite;

impl Factory for SQLite {
    type DB = Sqlite;

    fn build() -> PoolOptions<Self::DB> {
        SqlitePoolOptions::new()
    }
}

#[derive(Default, Debug)]
pub struct Params {
    pub min_conns: Option<u32>,
    pub max_conns: Option<u32>,
    pub conn_timeout: Option<Duration>,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
}

/// 生成 DB 连接池
///
/// # Examples
///
/// ```
/// // [MySQL] mysql://<username>:<password>@<host>:3306/<db>&charset=utf8mb4&parseTime=True&loc=Local
/// let x = sql::open::<sql::MySQL>("dsn", None).await;
///
/// // [PgSQL] postgres://<username>:<password>@<host>:5432/<db>?options=-c%20TimeZone%3DAsia/Shanghai
/// let x = sql::open::<sql::PgSQL>("dsn", None).await;
///
/// // [SQLite] sqlite://</path/test.db> || sqlite::memory:?cache=shared
/// let x = sql::open::<sql::SQLite>("dsn", None).await;
/// ```
pub async fn open<F>(dsn: String, opt: Option<Params>) -> anyhow::Result<Pool<F::DB>>
where
    F: Factory,
{
    let params = opt.unwrap_or_default();

    let pool = F::build()
        .min_connections(params.min_conns.unwrap_or(10))
        .max_connections(params.max_conns.unwrap_or(20))
        .acquire_timeout(params.conn_timeout.unwrap_or(Duration::from_secs(10)))
        .idle_timeout(params.idle_timeout.unwrap_or(Duration::from_secs(300)))
        .max_lifetime(params.max_lifetime.unwrap_or(Duration::from_secs(600)))
        .connect(&dsn)
        .await?;

    Ok(pool)
}

pub type Logger = Box<dyn Fn(String, Duration, Option<&anyhow::Error>) + Send + Sync + 'static>;

static SQL_LOGGER: OnceLock<Logger> = OnceLock::new();

/// 设置SQL日志
///
/// # Examples
///
/// ```
/// sql::set_sql_logger(|sql, cost, err| {
///     match err {
///         Some(e) => {
///             tracing::error!(sql = sql, cost_ms = cost.as_millis(), err = %e, "sql error");
///         }
///         None => {
///             if cost > Duration::from_millis(200) {
///                 tracing::warn!(sql = sql, cost_ms = cost.as_millis(), "slow sql");
///             } else {
///                 tracing::info!(sql = sql, cost_ms = cost.as_millis(), "sql");
///             }
///         }
///     }
/// })
/// ```
pub fn set_sql_logger<F>(f: F)
where
    F: Fn(String, Duration, Option<&anyhow::Error>) + Send + Sync + 'static,
{
    let _ = SQL_LOGGER.set(Box::new(f));
}

#[inline]
fn trace_sql(sql: String, cost: Duration, err: Option<&anyhow::Error>) {
    if let Some(logger) = SQL_LOGGER.get() {
        logger(sql, cost, err)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::sql;

    #[test]
    fn test_sql_logger() {
        sql::set_sql_logger(|sql, cost, err| match err {
            Some(e) => {
                tracing::error!(sql = sql, cost_ms = cost.as_millis(), err = %e, "sql error");
            }
            None => {
                if cost > Duration::from_millis(200) {
                    tracing::warn!(sql = sql, cost_ms = cost.as_millis(), "slow sql");
                } else {
                    tracing::info!(sql = sql, cost_ms = cost.as_millis(), "sql");
                }
            }
        })
    }
}
