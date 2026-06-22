pub mod connect;
pub mod ddl;
pub mod driver;
pub mod mysql;
pub mod nosql;
pub mod pool;
pub mod postgres;
pub mod sqlite;
pub mod sqlserver;

use serde::{Deserialize, Serialize};

use crate::db::connect::ConnectionConfig;
use crate::db::driver::Driver;
use crate::error::AppResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DbKind {
    Postgres,
    Mysql,
    Sqlite,
    Sqlserver,
    Redis,
    Mongodb,
}

/// Factory: pick a driver implementation from the connection's kind.
pub async fn open(cfg: &ConnectionConfig) -> AppResult<Box<dyn Driver>> {
    match cfg.kind {
        DbKind::Postgres => Ok(Box::new(postgres::PgDriver::connect(cfg).await?)),
        DbKind::Mysql => Ok(Box::new(mysql::MySqlDriver::connect(cfg).await?)),
        DbKind::Sqlite => Ok(Box::new(sqlite::SqliteDriver::connect(cfg).await?)),
        DbKind::Sqlserver => sqlserver::connect(cfg).await,
        DbKind::Redis | DbKind::Mongodb => nosql::connect(cfg).await,
    }
}
