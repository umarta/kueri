use async_trait::async_trait;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde_json::{json, Value};
use tiberius::{AuthMethod, Client, ColumnData, Config, EncryptionLevel, Row};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use crate::db::connect::ConnectionConfig;
use crate::db::ddl::Dialect;
use crate::db::driver::{ColumnInfo, Driver, QueryResult, SchemaInfo, TableInfo};
use crate::error::{AppError, AppResult};

// tiberius has no built-in pool; a desktop client uses one connection guarded by
// a mutex (queries need `&mut self`). Good enough for a single-user GUI.
type Tds = Client<Compat<TcpStream>>;

pub struct SqlServerDriver {
    client: Mutex<Tds>,
}

/// Open a SQL Server connection (sqlx dropped MSSQL — this uses `tiberius`).
pub async fn connect(cfg: &ConnectionConfig) -> AppResult<Box<dyn Driver>> {
    let mut config = Config::new();
    config.host(&cfg.host);
    config.port(if cfg.port == 0 { 1433 } else { cfg.port });
    if !cfg.database.is_empty() {
        config.database(&cfg.database);
    }
    config.authentication(AuthMethod::sql_server(&cfg.user, &cfg.password));
    config.encryption(if cfg.ssl {
        EncryptionLevel::Required
    } else {
        EncryptionLevel::NotSupported
    });
    // Desktop client: accept self-signed certs. TODO: expose a "verify cert" toggle.
    config.trust_cert();

    let tcp = TcpStream::connect(config.get_addr())
        .await
        .map_err(|e| AppError::Other(format!("sql server tcp: {e}")))?;
    tcp.set_nodelay(true).ok();

    let client = Client::connect(config, tcp.compat_write())
        .await
        .map_err(|e| AppError::Other(format!("sql server: {e}")))?;
    Ok(Box::new(SqlServerDriver {
        client: Mutex::new(client),
    }))
}

fn tds_err(e: tiberius::error::Error) -> AppError {
    AppError::Other(format!("sql server: {e}"))
}

/// Pull the first string column from each row of a (parameterized) query.
async fn string_column(client: &mut Tds, sql: &str, params: &[&str]) -> AppResult<Vec<String>> {
    let boxed: Vec<&dyn tiberius::ToSql> =
        params.iter().map(|p| p as &dyn tiberius::ToSql).collect();
    let rows = client
        .query(sql, &boxed)
        .await
        .map_err(tds_err)?
        .into_first_result()
        .await
        .map_err(tds_err)?;
    Ok(rows
        .iter()
        .filter_map(|r| r.try_get::<&str, _>(0).ok().flatten().map(str::to_string))
        .collect())
}

#[async_trait]
impl Driver for SqlServerDriver {
    fn dialect(&self) -> Dialect {
        Dialect::SqlServer
    }

    async fn list_schemas(&self) -> AppResult<Vec<SchemaInfo>> {
        let mut client = self.client.lock().await;
        let names = string_column(
            &mut client,
            "SELECT name FROM sys.schemas \
             WHERE name NOT IN ('guest','INFORMATION_SCHEMA','sys','db_owner', \
               'db_accessadmin','db_securityadmin','db_ddladmin','db_backupoperator', \
               'db_datareader','db_datawriter','db_denydatareader','db_denydatawriter') \
             ORDER BY name",
            &[],
        )
        .await?;
        Ok(names.into_iter().map(|name| SchemaInfo { name }).collect())
    }

    async fn list_tables(&self, schema: &str) -> AppResult<Vec<TableInfo>> {
        let mut client = self.client.lock().await;
        let rows = client
            .query(
                "SELECT TABLE_NAME, TABLE_TYPE FROM INFORMATION_SCHEMA.TABLES \
                 WHERE TABLE_SCHEMA = @P1 ORDER BY TABLE_NAME",
                &[&schema],
            )
            .await
            .map_err(tds_err)?
            .into_first_result()
            .await
            .map_err(tds_err)?;
        Ok(rows
            .iter()
            .map(|r| TableInfo {
                name: r
                    .try_get::<&str, _>(0)
                    .ok()
                    .flatten()
                    .unwrap_or("")
                    .to_string(),
                kind: r
                    .try_get::<&str, _>(1)
                    .ok()
                    .flatten()
                    .unwrap_or("BASE TABLE")
                    .to_string(),
            })
            .collect())
    }

    async fn list_columns(&self, schema: &str, table: &str) -> AppResult<Vec<ColumnInfo>> {
        let mut client = self.client.lock().await;
        let rows = client
            .query(
                "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE, COLUMN_DEFAULT \
                 FROM INFORMATION_SCHEMA.COLUMNS \
                 WHERE TABLE_SCHEMA = @P1 AND TABLE_NAME = @P2 ORDER BY ORDINAL_POSITION",
                &[&schema, &table],
            )
            .await
            .map_err(tds_err)?
            .into_first_result()
            .await
            .map_err(tds_err)?;
        Ok(rows
            .iter()
            .map(|r| ColumnInfo {
                name: r
                    .try_get::<&str, _>(0)
                    .ok()
                    .flatten()
                    .unwrap_or("")
                    .to_string(),
                data_type: r
                    .try_get::<&str, _>(1)
                    .ok()
                    .flatten()
                    .unwrap_or("")
                    .to_string(),
                nullable: r.try_get::<&str, _>(2).ok().flatten() == Some("YES"),
                default: r.try_get::<&str, _>(3).ok().flatten().map(str::to_string),
            })
            .collect())
    }

    async fn list_primary_keys(&self, schema: &str, table: &str) -> AppResult<Vec<String>> {
        let mut client = self.client.lock().await;
        string_column(
            &mut client,
            "SELECT kcu.COLUMN_NAME \
             FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc \
             JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu \
               ON tc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME \
              AND tc.TABLE_SCHEMA = kcu.TABLE_SCHEMA \
             WHERE tc.CONSTRAINT_TYPE = 'PRIMARY KEY' \
               AND tc.TABLE_SCHEMA = @P1 AND tc.TABLE_NAME = @P2 \
             ORDER BY kcu.ORDINAL_POSITION",
            &[schema, table],
        )
        .await
    }

    async fn run_query(&self, sql: &str) -> AppResult<QueryResult> {
        let mut client = self.client.lock().await;
        let rows = client
            .simple_query(sql)
            .await
            .map_err(tds_err)?
            .into_first_result()
            .await
            .map_err(tds_err)?;

        let columns: Vec<String> = rows
            .first()
            .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
            .unwrap_or_default();

        let mut out = Vec::with_capacity(rows.len());
        for row in &rows {
            let mut rec = Vec::with_capacity(columns.len());
            for (i, (_col, data)) in row.cells().enumerate() {
                rec.push(decode(row, i, data));
            }
            out.push(rec);
        }
        let row_count = out.len();
        Ok(QueryResult {
            columns,
            rows: out,
            row_count,
        })
    }

    async fn close(&self) {
        // tiberius closes on drop; nothing to do.
    }
}

fn ts<T: ToString>(r: tiberius::Result<Option<T>>) -> Value {
    match r {
        Ok(Some(v)) => Value::String(v.to_string()),
        _ => Value::Null,
    }
}

/// Map a tiberius cell to JSON. NULLs and exotic types fall back to null/text.
fn decode(row: &Row, i: usize, data: &ColumnData) -> Value {
    use ColumnData::*;
    match data {
        U8(Some(v)) => json!(v),
        I16(Some(v)) => json!(v),
        I32(Some(v)) => json!(v),
        I64(Some(v)) => json!(v),
        F32(Some(v)) => json!(v),
        F64(Some(v)) => json!(v),
        Bit(Some(v)) => json!(v),
        String(Some(s)) => Value::String(s.to_string()),
        Guid(Some(g)) => Value::String(g.to_string()),
        Numeric(Some(n)) => Value::String(n.to_string()),
        Xml(Some(x)) => Value::String(x.to_string()),
        Binary(Some(b)) => Value::String(b.iter().map(|x| format!("{x:02x}")).collect()),
        DateTime(Some(_)) | SmallDateTime(Some(_)) | DateTime2(Some(_)) => {
            ts(row.try_get::<NaiveDateTime, _>(i))
        }
        Date(Some(_)) => ts(row.try_get::<NaiveDate, _>(i)),
        Time(Some(_)) => ts(row.try_get::<NaiveTime, _>(i)),
        DateTimeOffset(Some(_)) => ts(row.try_get::<chrono::DateTime<Utc>, _>(i)),
        _ => Value::Null,
    }
}
