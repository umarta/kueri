use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use serde_json::Value;
use sqlx::pool::PoolConnection;
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::{Column, Executor, Postgres, Row, TypeInfo, ValueRef};
use tokio::sync::Mutex;

use crate::db::connect::ConnectionConfig;
use crate::db::ddl::Dialect;
use crate::db::driver::{
    ColumnInfo, Driver, ForeignKey, IndexInfo, ProcessInfo, QueryResult, RoleInfo, SchemaInfo,
    TableInfo,
};
use crate::error::{AppError, AppResult};

pub struct PgDriver {
    pool: PgPool,
    /// The pinned connection for an open manual transaction.
    txn: Mutex<Option<PoolConnection<Postgres>>>,
    in_txn: AtomicBool,
}

impl PgDriver {
    pub async fn connect(cfg: &ConnectionConfig) -> AppResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&cfg.pg_url())
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
impl Driver for PgDriver {
    fn dialect(&self) -> Dialect {
        Dialect::Postgres
    }

    async fn list_schemas(&self) -> AppResult<Vec<SchemaInfo>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT schema_name FROM information_schema.schemata \
             WHERE schema_name NOT IN ('pg_catalog','information_schema') \
             AND schema_name NOT LIKE 'pg_%' ORDER BY schema_name",
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
            "SELECT table_name, table_type FROM information_schema.tables \
             WHERE table_schema = $1 ORDER BY table_name",
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
        let rows: Vec<(String, String, String, Option<String>, String, Option<String>)> = sqlx::query_as(
            "SELECT c.column_name, c.data_type, c.is_nullable, c.column_default, c.udt_name, pgd.description \
             FROM information_schema.columns c \
             LEFT JOIN pg_catalog.pg_statio_all_tables st \
               ON st.schemaname = c.table_schema AND st.relname = c.table_name \
             LEFT JOIN pg_catalog.pg_description pgd \
               ON pgd.objoid = st.relid AND pgd.objsubid = c.ordinal_position \
             WHERE c.table_schema = $1 AND c.table_name = $2 ORDER BY c.ordinal_position",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        // Enum types → their labels (in sort order), keyed by type name.
        let enum_rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT t.typname, e.enumlabel FROM pg_type t \
             JOIN pg_enum e ON e.enumtypid = t.oid ORDER BY e.enumsortorder",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();
        let mut enums: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for (t, l) in enum_rows {
            enums.entry(t).or_default().push(l);
        }
        Ok(rows
            .into_iter()
            .map(
                |(name, data_type, is_nullable, default, udt_name, comment)| {
                    // For enums, information_schema reports "USER-DEFINED"; show the type
                    // name instead and attach its values.
                    let enum_values = enums.get(&udt_name).cloned().unwrap_or_default();
                    let data_type = if data_type == "USER-DEFINED" {
                        udt_name
                    } else {
                        data_type
                    };
                    ColumnInfo {
                        name,
                        data_type,
                        nullable: is_nullable == "YES",
                        default,
                        enum_values,
                        comment,
                    }
                },
            )
            .collect())
    }

    async fn list_primary_keys(&self, schema: &str, table: &str) -> AppResult<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT kcu.column_name \
             FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage kcu \
               ON tc.constraint_name = kcu.constraint_name \
              AND tc.table_schema = kcu.table_schema \
             WHERE tc.constraint_type = 'PRIMARY KEY' \
               AND tc.table_schema = $1 AND tc.table_name = $2 \
             ORDER BY kcu.ordinal_position",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(name,)| name).collect())
    }

    async fn list_foreign_keys(&self, schema: &str, table: &str) -> AppResult<Vec<ForeignKey>> {
        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT kcu.column_name, ccu.table_schema, ccu.table_name, ccu.column_name \
             FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage kcu \
               ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema \
             JOIN information_schema.constraint_column_usage ccu \
               ON ccu.constraint_name = tc.constraint_name AND ccu.table_schema = tc.table_schema \
             WHERE tc.constraint_type = 'FOREIGN KEY' \
               AND tc.table_schema = $1 AND tc.table_name = $2",
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
        let rows: Vec<(String, bool, String, Option<String>, String)> = sqlx::query_as(
            "SELECT i.relname, ix.indisunique, am.amname, \
                    pg_get_expr(ix.indpred, ix.indrelid), \
                    array_to_string(array_agg(a.attname ORDER BY x.ord), ',') \
             FROM pg_index ix \
             JOIN pg_class i ON i.oid = ix.indexrelid \
             JOIN pg_am am ON am.oid = i.relam \
             JOIN pg_class t ON t.oid = ix.indrelid \
             JOIN pg_namespace n ON n.oid = t.relnamespace \
             JOIN unnest(ix.indkey) WITH ORDINALITY AS x(attnum, ord) ON true \
             JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = x.attnum \
             WHERE n.nspname = $1 AND t.relname = $2 \
             GROUP BY i.relname, ix.indisunique, am.amname, ix.indpred, ix.indrelid \
             ORDER BY i.relname",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name, unique, method, predicate, cols)| IndexInfo {
                name,
                unique,
                method,
                predicate: predicate.unwrap_or_default(),
                columns: cols.split(',').map(|s| s.to_string()).collect(),
            })
            .collect())
    }

    async fn list_processes(&self) -> AppResult<Vec<ProcessInfo>> {
        let rows: Vec<(
            i32,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<i32>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT pid, usename, datname, state, \
                 EXTRACT(EPOCH FROM (now() - query_start))::int, query \
                 FROM pg_stat_activity WHERE pid <> pg_backend_pid() AND query <> '' \
                 ORDER BY query_start NULLS LAST",
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
                seconds: secs.unwrap_or(0) as i64,
                query: query.unwrap_or_default(),
            })
            .collect())
    }

    async fn kill_process(&self, pid: &str) -> AppResult<()> {
        let n: i32 = pid
            .parse()
            .map_err(|_| AppError::Other(format!("invalid pid: {pid}")))?;
        sqlx::query("SELECT pg_terminate_backend($1)")
            .bind(n)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_roles(&self) -> AppResult<Vec<RoleInfo>> {
        let rows: Vec<(String, bool, bool, bool)> = sqlx::query_as(
            "SELECT rolname, rolsuper, rolcreatedb, rolcanlogin FROM pg_roles ORDER BY rolname",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name, super_, createdb, login)| {
                let mut a = vec![];
                if super_ {
                    a.push("superuser");
                }
                if createdb {
                    a.push("createdb");
                }
                a.push(if login { "login" } else { "no-login" });
                RoleInfo {
                    name,
                    attributes: a.join(", "),
                }
            })
            .collect())
    }

    async fn view_definition(&self, schema: &str, name: &str) -> AppResult<String> {
        let row: (String,) =
            sqlx::query_as("SELECT pg_get_viewdef(format('%I.%I', $1, $2)::regclass, true)")
                .bind(schema)
                .bind(name)
                .fetch_one(&self.pool)
                .await?;
        let q = crate::db::ddl::qualify(Dialect::Postgres, schema, name);
        Ok(format!(
            "CREATE OR REPLACE VIEW {q} AS\n{};",
            row.0.trim_end().trim_end_matches(';')
        ))
    }

    async fn list_objects(&self, schema: &str, kind: &str) -> AppResult<Vec<String>> {
        let sql = match kind {
            "routine" => {
                "SELECT routine_name FROM information_schema.routines \
                          WHERE routine_schema = $1 AND routine_type IN ('FUNCTION','PROCEDURE') \
                          ORDER BY routine_name"
            }
            "trigger" => {
                "SELECT DISTINCT trigger_name FROM information_schema.triggers \
                          WHERE trigger_schema = $1 ORDER BY trigger_name"
            }
            "sequence" => {
                "SELECT sequence_name FROM information_schema.sequences \
                           WHERE sequence_schema = $1 ORDER BY sequence_name"
            }
            _ => return Ok(vec![]),
        };
        let rows: Vec<(String,)> = sqlx::query_as(sql)
            .bind(schema)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|(n,)| n).collect())
    }

    async fn object_definition(&self, schema: &str, name: &str, kind: &str) -> AppResult<String> {
        match kind {
            "routine" => {
                let row: (String,) = sqlx::query_as(
                    "SELECT pg_get_functiondef(p.oid) FROM pg_proc p \
                     JOIN pg_namespace n ON n.oid = p.pronamespace \
                     WHERE n.nspname = $1 AND p.proname = $2 LIMIT 1",
                )
                .bind(schema)
                .bind(name)
                .fetch_one(&self.pool)
                .await?;
                Ok(format!("{};", row.0.trim_end().trim_end_matches(';')))
            }
            "trigger" => {
                let row: (String,) = sqlx::query_as(
                    "SELECT pg_get_triggerdef(t.oid) FROM pg_trigger t \
                     JOIN pg_class c ON c.oid = t.tgrelid \
                     JOIN pg_namespace n ON n.oid = c.relnamespace \
                     WHERE n.nspname = $1 AND t.tgname = $2 AND NOT t.tgisinternal LIMIT 1",
                )
                .bind(schema)
                .bind(name)
                .fetch_one(&self.pool)
                .await?;
                Ok(format!("{};", row.0.trim_end().trim_end_matches(';')))
            }
            "sequence" => {
                let r: (Option<i64>, Option<i64>, Option<i64>, Option<i64>) = sqlx::query_as(
                    "SELECT start_value, increment_by, min_value, max_value \
                     FROM pg_sequences WHERE schemaname = $1 AND sequencename = $2",
                )
                .bind(schema)
                .bind(name)
                .fetch_one(&self.pool)
                .await?;
                Ok(format!(
                    "CREATE SEQUENCE {}.{}\n  START WITH {}\n  INCREMENT BY {}\n  MINVALUE {}\n  MAXVALUE {};",
                    schema,
                    name,
                    r.0.unwrap_or(1),
                    r.1.unwrap_or(1),
                    r.2.map(|v| v.to_string()).unwrap_or_else(|| "NO MINVALUE".into()),
                    r.3.map(|v| v.to_string()).unwrap_or_else(|| "NO MAXVALUE".into()),
                ))
            }
            _ => Err(AppError::Other(format!("unknown object kind: {kind}"))),
        }
    }

    async fn run_query(&self, sql: &str) -> AppResult<QueryResult> {
        // In a manual transaction, run on the pinned connection; else on the pool.
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
            // Empty result: pull column names from the prepared statement so the
            // grid still shows headers and the filter bar keeps its column list.
            match (&self.pool).describe(sql).await {
                Ok(d) => d.columns().iter().map(|c| c.name().to_string()).collect(),
                Err(_) => Vec::new(),
            }
        };
        let mut out = Vec::with_capacity(rows.len());
        for row in &rows {
            let mut rec = Vec::with_capacity(row.columns().len());
            for (i, col) in row.columns().iter().enumerate() {
                rec.push(decode(row, i, col.type_info().name()));
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
        sqlx::query("BEGIN").execute(&mut *conn).await?;
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

fn decode(row: &PgRow, idx: usize, type_name: &str) -> Value {
    if let Ok(raw) = row.try_get_raw(idx) {
        if raw.is_null() {
            return Value::Null;
        }
    }
    macro_rules! j {
        ($($t:ty),*) => {{ $( if let Ok(v) = row.try_get::<$t,_>(idx) {
            return serde_json::to_value(v).unwrap_or(Value::Null); } )* }};
    }
    match type_name {
        "BOOL" => j!(bool),
        "INT2" => j!(i16),
        "INT4" => j!(i32),
        // i64 exceeds JS's safe integer range — emit as a string so big IDs keep
        // full precision (otherwise WHERE "id" = <rounded> matches no rows on edit).
        "INT8" => {
            if let Ok(v) = row.try_get::<i64, _>(idx) {
                return Value::String(v.to_string());
            }
        }
        "FLOAT4" => j!(f32),
        "FLOAT8" => j!(f64),
        "JSON" | "JSONB" => j!(serde_json::Value),
        "TEXT" | "VARCHAR" | "BPCHAR" | "NAME" => j!(String),
        "NUMERIC" => {
            if let Ok(v) = row.try_get::<sqlx::types::BigDecimal, _>(idx) {
                return Value::String(v.to_string());
            }
        }
        "UUID" => {
            if let Ok(v) = row.try_get::<sqlx::types::Uuid, _>(idx) {
                return Value::String(v.to_string());
            }
        }
        "TIMESTAMP" => {
            if let Ok(v) = row.try_get::<sqlx::types::chrono::NaiveDateTime, _>(idx) {
                return Value::String(v.to_string());
            }
        }
        "TIMESTAMPTZ" => {
            if let Ok(v) =
                row.try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>(idx)
            {
                return Value::String(v.to_rfc3339());
            }
        }
        "DATE" => {
            if let Ok(v) = row.try_get::<sqlx::types::chrono::NaiveDate, _>(idx) {
                return Value::String(v.to_string());
            }
        }
        _ => {}
    }
    if let Ok(v) = row.try_get::<String, _>(idx) {
        return Value::String(v);
    }
    Value::Null
}
