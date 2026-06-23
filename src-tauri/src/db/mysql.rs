use async_trait::async_trait;
use serde_json::Value;
use sqlx::mysql::{MySqlPool, MySqlPoolOptions, MySqlRow};
use sqlx::{Column, Row, ValueRef};

use crate::db::connect::ConnectionConfig;
use crate::db::ddl::Dialect;
use crate::db::driver::{
    ColumnInfo, Driver, ForeignKey, IndexInfo, QueryResult, SchemaInfo, TableInfo,
};
use crate::error::AppResult;

pub struct MySqlDriver {
    pool: MySqlPool,
}

impl MySqlDriver {
    pub async fn connect(cfg: &ConnectionConfig) -> AppResult<Self> {
        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&cfg.mysql_url())
            .await?;
        sqlx::query("SELECT 1").execute(&pool).await?;
        Ok(Self { pool })
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
        let rows: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
            "SELECT CAST(column_name AS CHAR), CAST(data_type AS CHAR), CAST(is_nullable AS CHAR), CAST(column_default AS CHAR) \
             FROM information_schema.columns \
             WHERE table_schema = ? AND table_name = ? ORDER BY ordinal_position",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name, data_type, is_nullable, default)| ColumnInfo {
                name,
                data_type,
                nullable: is_nullable == "YES",
                default,
            })
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
        let rows: Vec<(String, i64, String, i64)> = sqlx::query_as(
            "SELECT CAST(index_name AS CHAR), CAST(non_unique AS SIGNED), \
                    CAST(column_name AS CHAR), seq_in_index \
             FROM information_schema.statistics \
             WHERE table_schema = ? AND table_name = ? \
             ORDER BY index_name, seq_in_index",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        let mut out: Vec<IndexInfo> = Vec::new();
        for (name, non_unique, col, _seq) in rows {
            if let Some(ix) = out.iter_mut().find(|i| i.name == name) {
                ix.columns.push(col);
            } else {
                out.push(IndexInfo {
                    name,
                    unique: non_unique == 0,
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

    async fn run_query(&self, sql: &str) -> AppResult<QueryResult> {
        let rows = sqlx::query(sql).fetch_all(&self.pool).await?;
        let columns = rows
            .first()
            .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
            .unwrap_or_default();
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

    async fn close(&self) {
        self.pool.close().await;
    }
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
