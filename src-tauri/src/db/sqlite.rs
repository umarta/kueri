use async_trait::async_trait;
use serde_json::Value;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions, SqliteRow};
use sqlx::{Column, Executor, Row, ValueRef};

use crate::db::connect::ConnectionConfig;
use crate::db::ddl::Dialect;
use crate::db::driver::{
    ColumnInfo, Driver, ForeignKey, IndexInfo, QueryResult, SchemaInfo, TableInfo,
};
use crate::error::AppResult;

pub struct SqliteDriver {
    pool: SqlitePool,
}

impl SqliteDriver {
    pub async fn connect(cfg: &ConnectionConfig) -> AppResult<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(3)
            .connect(&cfg.sqlite_url())
            .await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl Driver for SqliteDriver {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }

    async fn list_schemas(&self) -> AppResult<Vec<SchemaInfo>> {
        // SQLite has no schemas; expose a single logical one.
        Ok(vec![SchemaInfo {
            name: "main".into(),
        }])
    }

    async fn list_tables(&self, _schema: &str) -> AppResult<Vec<TableInfo>> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT name, type FROM sqlite_master \
             WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name, kind)| TableInfo { name, kind })
            .collect())
    }

    async fn list_columns(&self, _schema: &str, table: &str) -> AppResult<Vec<ColumnInfo>> {
        // PRAGMA can't be parameterized; table name is from our own sidebar.
        let q = format!("PRAGMA table_info(\"{}\")", table.replace('"', "\"\""));
        let rows = sqlx::query(&q).fetch_all(&self.pool).await?;
        let mut cols = Vec::new();
        for r in &rows {
            let name: String = r.try_get("name").unwrap_or_default();
            let data_type: String = r.try_get("type").unwrap_or_default();
            let notnull: i64 = r.try_get("notnull").unwrap_or(0);
            let default: Option<String> = r.try_get("dflt_value").ok();
            cols.push(ColumnInfo {
                name,
                data_type,
                nullable: notnull == 0,
                default,
                enum_values: vec![],
            });
        }
        Ok(cols)
    }

    async fn list_primary_keys(&self, _schema: &str, table: &str) -> AppResult<Vec<String>> {
        // PRAGMA can't be parameterized; table name is from our own sidebar.
        let q = format!("PRAGMA table_info(\"{}\")", table.replace('"', "\"\""));
        let rows = sqlx::query(&q).fetch_all(&self.pool).await?;
        // `pk` is 0 for non-key columns, else the 1-based position in the key.
        let mut keyed: Vec<(i64, String)> = Vec::new();
        for r in &rows {
            let pk: i64 = r.try_get("pk").unwrap_or(0);
            if pk > 0 {
                let name: String = r.try_get("name").unwrap_or_default();
                keyed.push((pk, name));
            }
        }
        keyed.sort_by_key(|(pos, _)| *pos);
        Ok(keyed.into_iter().map(|(_, name)| name).collect())
    }

    async fn list_foreign_keys(&self, _schema: &str, table: &str) -> AppResult<Vec<ForeignKey>> {
        // PRAGMA can't be parameterized; table name is from our own sidebar.
        let q = format!(
            "PRAGMA foreign_key_list(\"{}\")",
            table.replace('"', "\"\"")
        );
        let rows = sqlx::query(&q).fetch_all(&self.pool).await?;
        let mut out = Vec::new();
        for r in &rows {
            let column: String = r.try_get("from").unwrap_or_default();
            let ref_table: String = r.try_get("table").unwrap_or_default();
            // `to` is NULL when the FK targets the referenced table's primary key.
            let ref_column: Option<String> = r.try_get("to").ok();
            out.push(ForeignKey {
                column,
                ref_schema: "main".into(),
                ref_table,
                ref_column: ref_column.unwrap_or_default(),
            });
        }
        Ok(out)
    }

    async fn list_indexes(&self, _schema: &str, table: &str) -> AppResult<Vec<IndexInfo>> {
        let q = format!("PRAGMA index_list(\"{}\")", table.replace('"', "\"\""));
        let rows = sqlx::query(&q).fetch_all(&self.pool).await?;
        let mut out = Vec::new();
        for r in &rows {
            let name: String = r.try_get("name").unwrap_or_default();
            let unique: i64 = r.try_get("unique").unwrap_or(0);
            let iq = format!("PRAGMA index_info(\"{}\")", name.replace('"', "\"\""));
            let irows = sqlx::query(&iq).fetch_all(&self.pool).await?;
            let columns = irows
                .iter()
                .map(|ir| ir.try_get::<String, _>("name").unwrap_or_default())
                .collect();
            out.push(IndexInfo {
                name,
                unique: unique == 1,
                method: "btree".into(),
                predicate: String::new(),
                columns,
            });
        }
        Ok(out)
    }

    async fn table_ddl(&self, _schema: &str, table: &str) -> AppResult<String> {
        // sqlite_master stores the original CREATE statement verbatim.
        let row: (Option<String>,) = sqlx::query_as(
            "SELECT sql FROM sqlite_master WHERE name = ? AND type IN ('table','view')",
        )
        .bind(table)
        .fetch_one(&self.pool)
        .await?;
        Ok(row
            .0
            .map(|s| format!("{};", s.trim_end_matches(';')))
            .unwrap_or_default())
    }

    async fn run_query(&self, sql: &str) -> AppResult<QueryResult> {
        let rows = sqlx::query(sql).fetch_all(&self.pool).await?;
        let columns: Vec<String> = if let Some(r) = rows.first() {
            r.columns().iter().map(|c| c.name().to_string()).collect()
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

    async fn close(&self) {
        self.pool.close().await;
    }
}

fn decode(row: &SqliteRow, idx: usize) -> Value {
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
    if let Ok(v) = row.try_get::<String, _>(idx) {
        return Value::String(v);
    }
    Value::Null
}
