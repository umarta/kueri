use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use serde_json::Value;
use sqlx::mysql::{MySqlPool, MySqlPoolOptions, MySqlRow};
use sqlx::pool::PoolConnection;
use sqlx::{Column, Executor, MySql, Row, ValueRef};
use tokio::sync::Mutex;

use crate::db::connect::ConnectionConfig;
use crate::db::ddl::Dialect;
use crate::db::driver::{
    ColumnInfo, Driver, ForeignKey, IndexInfo, ProcessInfo, QueryResult, RoleInfo, SchemaInfo,
    TableInfo,
};
use crate::error::{AppError, AppResult};

pub struct MySqlDriver {
    pool: MySqlPool,
    txn: Mutex<Option<PoolConnection<MySql>>>,
    in_txn: AtomicBool,
}

impl MySqlDriver {
    pub async fn connect(cfg: &ConnectionConfig) -> AppResult<Self> {
        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&cfg.mysql_url())
            .await?;
        sqlx::query("SELECT 1").execute(&pool).await?;
        Ok(Self {
            pool,
            txn: Mutex::new(None),
            in_txn: AtomicBool::new(false),
        })
    }
}

#[async_trait]
impl Driver for MySqlDriver {
    fn dialect(&self) -> Dialect {
        Dialect::MySql
    }

    async fn list_schemas(&self) -> AppResult<Vec<SchemaInfo>> {
        // In MySQL a "schema" is a database.
        let rows: Vec<(String,)> = sqlx::query_as(
            // CAST to CHAR: MySQL's information_schema columns can come back as a
            // binary type that sqlx refuses to decode into String, which would
            // surface as an error (and an empty table list). CHAR is always text.
            "SELECT CAST(schema_name AS CHAR) FROM information_schema.schemata \
             WHERE schema_name NOT IN ('mysql','sys','performance_schema','information_schema') \
             ORDER BY schema_name",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name,)| SchemaInfo { name })
            .collect())
    }

    async fn list_tables(&self, schema: &str) -> AppResult<Vec<TableInfo>> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT CAST(table_name AS CHAR), CAST(table_type AS CHAR) FROM information_schema.tables \
             WHERE table_schema = ? ORDER BY table_name",
        )
        .bind(schema)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name, kind)| TableInfo { name, kind })
            .collect())
    }

    async fn list_columns(&self, schema: &str, table: &str) -> AppResult<Vec<ColumnInfo>> {
        let rows: Vec<(String, String, String, Option<String>, String, String)> = sqlx::query_as(
            "SELECT CAST(column_name AS CHAR), CAST(data_type AS CHAR), CAST(is_nullable AS CHAR), \
                    CAST(column_default AS CHAR), CAST(column_type AS CHAR), CAST(column_comment AS CHAR) \
             FROM information_schema.columns \
             WHERE table_schema = ? AND table_name = ? ORDER BY ordinal_position",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(name, data_type, is_nullable, default, column_type, comment)| ColumnInfo {
                    enum_values: parse_enum(&column_type),
                    name,
                    data_type,
                    nullable: is_nullable == "YES",
                    default,
                    comment: if comment.is_empty() {
                        None
                    } else {
                        Some(comment)
                    },
                },
            )
            .collect())
    }

    async fn list_primary_keys(&self, schema: &str, table: &str) -> AppResult<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT CAST(column_name AS CHAR) FROM information_schema.columns \
             WHERE table_schema = ? AND table_name = ? AND column_key = 'PRI' \
             ORDER BY ordinal_position",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(name,)| name).collect())
    }

    async fn list_foreign_keys(&self, schema: &str, table: &str) -> AppResult<Vec<ForeignKey>> {
        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT CAST(column_name AS CHAR), CAST(referenced_table_schema AS CHAR), \
                    CAST(referenced_table_name AS CHAR), CAST(referenced_column_name AS CHAR) \
             FROM information_schema.key_column_usage \
             WHERE table_schema = ? AND table_name = ? AND referenced_table_name IS NOT NULL",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(column, ref_schema, ref_table, ref_column)| ForeignKey {
                column,
                ref_schema,
                ref_table,
                ref_column,
            })
            .collect())
    }

    async fn list_indexes(&self, schema: &str, table: &str) -> AppResult<Vec<IndexInfo>> {
        let rows: Vec<(String, i64, String, i64, String)> = sqlx::query_as(
            "SELECT CAST(index_name AS CHAR), CAST(non_unique AS SIGNED), \
                    CAST(column_name AS CHAR), seq_in_index, CAST(index_type AS CHAR) \
             FROM information_schema.statistics \
             WHERE table_schema = ? AND table_name = ? \
             ORDER BY index_name, seq_in_index",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        let mut out: Vec<IndexInfo> = Vec::new();
        for (name, non_unique, col, _seq, method) in rows {
            if let Some(ix) = out.iter_mut().find(|i| i.name == name) {
                ix.columns.push(col);
            } else {
                out.push(IndexInfo {
                    name,
                    unique: non_unique == 0,
                    method,
                    predicate: String::new(),
                    columns: vec![col],
                });
            }
        }
        Ok(out)
    }

    async fn table_ddl(&self, schema: &str, table: &str) -> AppResult<String> {
        let q = format!(
            "SHOW CREATE TABLE `{}`.`{}`",
            schema.replace('`', "``"),
            table.replace('`', "``")
        );
        // Returns (Table, Create Table).
        let row: (String, String) = sqlx::query_as(&q).fetch_one(&self.pool).await?;
        Ok(format!("{};", row.1))
    }

    async fn list_processes(&self) -> AppResult<Vec<ProcessInfo>> {
        let rows: Vec<(
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            i64,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT id, CAST(user AS CHAR), CAST(db AS CHAR), CAST(state AS CHAR), \
                 CAST(time AS SIGNED), CAST(info AS CHAR) \
                 FROM information_schema.processlist WHERE info IS NOT NULL ORDER BY time DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(pid, user, db, state, secs, query)| ProcessInfo {
                pid: pid.to_string(),
                user: user.unwrap_or_default(),
                database: db.unwrap_or_default(),
                state: state.unwrap_or_default(),
                seconds: secs,
                query: query.unwrap_or_default(),
            })
            .collect())
    }

    async fn kill_process(&self, pid: &str) -> AppResult<()> {
        let n: i64 = pid
            .parse()
            .map_err(|_| AppError::Other(format!("invalid id: {pid}")))?;
        sqlx::query(&format!("KILL {n}"))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_roles(&self) -> AppResult<Vec<RoleInfo>> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT CAST(user AS CHAR), CAST(host AS CHAR) FROM mysql.user ORDER BY user",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();
        Ok(rows
            .into_iter()
            .map(|(name, host)| RoleInfo {
                name,
                attributes: format!("@{host}"),
            })
            .collect())
    }

    async fn view_definition(&self, schema: &str, name: &str) -> AppResult<String> {
        let q = format!(
            "SHOW CREATE VIEW `{}`.`{}`",
            schema.replace('`', "``"),
            name.replace('`', "``")
        );
        // Returns (View, Create View, character_set_client, collation_connection).
        let row: (String, String, String, String) =
            sqlx::query_as(&q).fetch_one(&self.pool).await?;
        Ok(format!("{};", row.1.trim_end().trim_end_matches(';')))
    }

    async fn run_query(&self, sql: &str) -> AppResult<QueryResult> {
        let (rows, empty_cols) = if self.in_txn.load(Ordering::Relaxed) {
            let mut guard = self.txn.lock().await;
            match guard.as_mut() {
                Some(conn) => {
                    let rows = sqlx::query(sql).fetch_all(&mut **conn).await?;
                    let cols = if rows.is_empty() {
                        (&mut **conn)
                            .describe(sql)
                            .await
                            .map(|d| d.columns().iter().map(|c| c.name().to_string()).collect())
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    (rows, cols)
                }
                None => (sqlx::query(sql).fetch_all(&self.pool).await?, Vec::new()),
            }
        } else {
            (sqlx::query(sql).fetch_all(&self.pool).await?, Vec::new())
        };
        let columns: Vec<String> = if let Some(r) = rows.first() {
            r.columns().iter().map(|c| c.name().to_string()).collect()
        } else if !empty_cols.is_empty() {
            empty_cols
        } else {
            match (&self.pool).describe(sql).await {
                Ok(d) => d.columns().iter().map(|c| c.name().to_string()).collect(),
                Err(_) => Vec::new(),
            }
        };
        let mut out = Vec::with_capacity(rows.len());
        for row in &rows {
            let mut rec = Vec::with_capacity(row.columns().len());
            for i in 0..row.columns().len() {
                rec.push(decode(row, i));
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

    async fn begin(&self) -> AppResult<()> {
        let mut g = self.txn.lock().await;
        if g.is_some() {
            return Err(AppError::Other("Already in a transaction.".into()));
        }
        let mut conn = self.pool.acquire().await?;
        sqlx::query("START TRANSACTION").execute(&mut *conn).await?;
        *g = Some(conn);
        self.in_txn.store(true, Ordering::Relaxed);
        Ok(())
    }
    async fn commit(&self) -> AppResult<()> {
        let mut g = self.txn.lock().await;
        if let Some(mut conn) = g.take() {
            sqlx::query("COMMIT").execute(&mut *conn).await?;
        }
        self.in_txn.store(false, Ordering::Relaxed);
        Ok(())
    }
    async fn rollback(&self) -> AppResult<()> {
        let mut g = self.txn.lock().await;
        if let Some(mut conn) = g.take() {
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;
        }
        self.in_txn.store(false, Ordering::Relaxed);
        Ok(())
    }

    async fn close(&self) {
        self.pool.close().await;
    }
}

/// Parse the values out of a MySQL `enum('a','b',...)` / `set('a','b')` column type.
fn parse_enum(column_type: &str) -> Vec<String> {
    let lower = column_type.to_ascii_lowercase();
    if !(lower.starts_with("enum(") || lower.starts_with("set(")) {
        return vec![];
    }
    let Some(open) = column_type.find('(') else {
        return vec![];
    };
    let close = column_type.rfind(')').unwrap_or(column_type.len());
    let inner = &column_type[open + 1..close];
    let mut out = Vec::new();
    let mut chars = inner.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == '\'' {
            chars.next();
            let mut s = String::new();
            while let Some(ch) = chars.next() {
                if ch == '\'' {
                    if chars.peek() == Some(&'\'') {
                        s.push('\'');
                        chars.next();
                    } else {
                        break;
                    }
                } else {
                    s.push(ch);
                }
            }
            out.push(s);
        } else {
            chars.next();
        }
    }
    out
}

/// Best-effort decode by trying common types in order.
fn decode(row: &MySqlRow, idx: usize) -> Value {
    if let Ok(raw) = row.try_get_raw(idx) {
        if raw.is_null() {
            return Value::Null;
        }
    }
    if let Ok(v) = row.try_get::<i64, _>(idx) {
        // Beyond JS's safe integer range, emit as a string to keep full precision.
        if v.abs() > 9_007_199_254_740_991 {
            return Value::String(v.to_string());
        }
        return Value::from(v);
    }
    // Unsigned integers (e.g. `bigint unsigned`) don't fit i64; try u64 BEFORE bool,
    // otherwise sqlx happily decodes any integer as a bool (non-zero → true).
    if let Ok(v) = row.try_get::<u64, _>(idx) {
        if v > 9_007_199_254_740_991 {
            return Value::String(v.to_string());
        }
        return Value::from(v);
    }
    if let Ok(v) = row.try_get::<f64, _>(idx) {
        return Value::from(v);
    }
    if let Ok(v) = row.try_get::<bool, _>(idx) {
        return Value::from(v);
    }
    if let Ok(v) = row.try_get::<sqlx::types::BigDecimal, _>(idx) {
        return Value::String(v.to_string());
    }
    if let Ok(v) = row.try_get::<sqlx::types::chrono::NaiveDateTime, _>(idx) {
        return Value::String(v.to_string());
    }
    if let Ok(v) = row.try_get::<serde_json::Value, _>(idx) {
        return v;
    }
    if let Ok(v) = row.try_get::<String, _>(idx) {
        return Value::String(v);
    }
    Value::Null
}
