use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;

use crate::db::ddl::{self, ColumnDef, Dialect};
use crate::error::{AppError, AppResult};

#[derive(Serialize)]
pub struct SchemaInfo {
    pub name: String,
}

#[derive(Serialize)]
pub struct TableInfo {
    pub name: String,
    pub kind: String,
}

#[derive(Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default: Option<String>,
    /// Allowed values for enum columns (empty for non-enums); powers a dropdown editor.
    #[serde(default)]
    pub enum_values: Vec<String>,
    /// Column comment / description, if any.
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Serialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub row_count: usize,
}

/// A foreign-key edge from a column to the referenced table column.
#[derive(Serialize)]
pub struct ForeignKey {
    pub column: String,
    pub ref_schema: String,
    pub ref_table: String,
    pub ref_column: String,
}

/// An index on a table.
#[derive(Serialize)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    /// Access method (btree/hash/gin/…); shown as "index_algorithm".
    pub method: String,
    /// Partial-index predicate (empty when not partial); shown as "condition".
    pub predicate: String,
}

/// A running server session/query (Server Monitor).
#[derive(Serialize)]
pub struct ProcessInfo {
    pub pid: String,
    pub user: String,
    pub database: String,
    pub state: String,
    pub seconds: i64,
    pub query: String,
}

/// A database role/user.
#[derive(Serialize)]
pub struct RoleInfo {
    pub name: String,
    pub attributes: String,
}

/// Every relational backend implements this. The UI and Tauri commands only
/// ever talk to `dyn Driver`, never to a concrete database. This is what keeps
/// the app simple while supporting many databases.
#[async_trait]
pub trait Driver: Send + Sync {
    async fn list_schemas(&self) -> AppResult<Vec<SchemaInfo>>;
    async fn list_tables(&self, schema: &str) -> AppResult<Vec<TableInfo>>;
    async fn list_columns(&self, schema: &str, table: &str) -> AppResult<Vec<ColumnInfo>>;
    /// Primary-key column names in key order. Empty when the table has no PK —
    /// the frontend then falls back to a full-row WHERE for edits.
    async fn list_primary_keys(&self, schema: &str, table: &str) -> AppResult<Vec<String>>;
    async fn run_query(&self, sql: &str) -> AppResult<QueryResult>;
    async fn close(&self);

    /// Foreign keys declared on a table. Default empty (engines without FK
    /// metadata); the relational drivers override it.
    async fn list_foreign_keys(&self, _schema: &str, _table: &str) -> AppResult<Vec<ForeignKey>> {
        Ok(vec![])
    }

    /// Indexes on a table. Default empty; relational drivers override it.
    async fn list_indexes(&self, _schema: &str, _table: &str) -> AppResult<Vec<IndexInfo>> {
        Ok(vec![])
    }

    /// Running sessions/queries (Server Monitor). Default empty.
    async fn list_processes(&self) -> AppResult<Vec<ProcessInfo>> {
        Ok(vec![])
    }
    /// Terminate a session by its id. Default unsupported.
    async fn kill_process(&self, _pid: &str) -> AppResult<()> {
        Err(AppError::Other(
            "Killing sessions isn't supported for this database.".into(),
        ))
    }
    /// Database roles/users. Default empty.
    async fn list_roles(&self) -> AppResult<Vec<RoleInfo>> {
        Ok(vec![])
    }

    /// Create / drop a schema (Postgres) or database (MySQL).
    async fn create_schema(&self, name: &str) -> AppResult<()> {
        self.run_query(&ddl::create_schema(self.dialect(), name))
            .await?;
        Ok(())
    }
    async fn drop_schema(&self, name: &str) -> AppResult<()> {
        self.run_query(&ddl::drop_schema(self.dialect(), name))
            .await?;
        Ok(())
    }
    async fn create_index(
        &self,
        schema: &str,
        table: &str,
        name: &str,
        columns: &[String],
        unique: bool,
    ) -> AppResult<()> {
        self.run_query(&ddl::create_index(
            self.dialect(),
            schema,
            table,
            name,
            columns,
            unique,
        ))
        .await?;
        Ok(())
    }
    async fn drop_index(&self, schema: &str, table: &str, name: &str) -> AppResult<()> {
        self.run_query(&ddl::drop_index(self.dialect(), schema, table, name))
            .await?;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    async fn add_foreign_key(
        &self,
        schema: &str,
        table: &str,
        column: &str,
        ref_table: &str,
        ref_column: &str,
        name: &str,
        validate: bool,
    ) -> AppResult<()> {
        if self.dialect() == Dialect::Sqlite {
            return Err(AppError::Other(
                "SQLite can't add a foreign key to an existing table (requires a table rebuild)."
                    .into(),
            ));
        }
        self.run_query(&ddl::add_foreign_key(
            self.dialect(),
            schema,
            table,
            column,
            ref_table,
            ref_column,
            name,
            validate,
        ))
        .await?;
        Ok(())
    }

    // ── DDL ──────────────────────────────────────────────────────────────────
    // Each driver reports its SQL dialect; the default methods below build the
    // right DDL and run it. This keeps every database difference in `ddl.rs`,
    // so the UI just calls these (never branching on database type).

    /// The SQL dialect this driver speaks (drives DDL generation).
    fn dialect(&self) -> Dialect;

    async fn create_table(&self, schema: &str, name: &str, columns: &[ColumnDef]) -> AppResult<()> {
        self.run_query(&ddl::create_table(self.dialect(), schema, name, columns))
            .await?;
        Ok(())
    }
    async fn drop_table(&self, schema: &str, table: &str) -> AppResult<()> {
        self.run_query(&ddl::drop_table(self.dialect(), schema, table))
            .await?;
        Ok(())
    }
    async fn rename_table(&self, schema: &str, old: &str, new: &str) -> AppResult<()> {
        self.run_query(&ddl::rename_table(self.dialect(), schema, old, new))
            .await?;
        Ok(())
    }
    async fn truncate_table(&self, schema: &str, table: &str) -> AppResult<()> {
        self.run_query(&ddl::truncate_table(self.dialect(), schema, table))
            .await?;
        Ok(())
    }
    async fn duplicate_table(&self, schema: &str, src: &str, dst: &str) -> AppResult<()> {
        self.run_query(&ddl::duplicate_table(self.dialect(), schema, src, dst))
            .await?;
        Ok(())
    }
    async fn add_column(&self, schema: &str, table: &str, column: &ColumnDef) -> AppResult<()> {
        self.run_query(&ddl::add_column(self.dialect(), schema, table, column))
            .await?;
        Ok(())
    }
    async fn drop_column(&self, schema: &str, table: &str, column: &str) -> AppResult<()> {
        self.run_query(&ddl::drop_column(self.dialect(), schema, table, column))
            .await?;
        Ok(())
    }
    async fn rename_column(
        &self,
        schema: &str,
        table: &str,
        old: &str,
        new: &str,
    ) -> AppResult<()> {
        self.run_query(&ddl::rename_column(self.dialect(), schema, table, old, new))
            .await?;
        Ok(())
    }
    async fn change_column_type(
        &self,
        schema: &str,
        table: &str,
        column: &str,
        new_type: &str,
        not_null: bool,
    ) -> AppResult<()> {
        if self.dialect() == Dialect::Sqlite {
            return Err(AppError::Other(
                "SQLite can't change a column type in place (requires a table rebuild).".into(),
            ));
        }
        self.run_query(&ddl::change_column_type(
            self.dialect(),
            schema,
            table,
            column,
            new_type,
            not_null,
        ))
        .await?;
        Ok(())
    }
    async fn set_column_nullable(
        &self,
        schema: &str,
        table: &str,
        column: &str,
        current_type: &str,
        not_null: bool,
    ) -> AppResult<()> {
        if self.dialect() == Dialect::Sqlite {
            return Err(AppError::Other(
                "SQLite can't change a column's nullability in place (requires a table rebuild)."
                    .into(),
            ));
        }
        self.run_query(&ddl::set_column_nullable(
            self.dialect(),
            schema,
            table,
            column,
            current_type,
            not_null,
        ))
        .await?;
        Ok(())
    }

    /// `CREATE TABLE` text for a table. The default reconstructs it from the
    /// column list + primary key (used by Postgres, which has no `SHOW CREATE`);
    /// MySQL and SQLite override this with their native, exact statements.
    async fn table_ddl(&self, schema: &str, table: &str) -> AppResult<String> {
        let d = self.dialect();
        let cols = self.list_columns(schema, table).await?;
        let pks = self.list_primary_keys(schema, table).await?;
        if cols.is_empty() {
            return Err(AppError::Other(format!(
                "Table {table} not found or has no columns."
            )));
        }
        let mut lines: Vec<String> = cols
            .iter()
            .map(|c| {
                let mut l = format!("  {} {}", ddl::ident(d, &c.name), c.data_type);
                if !c.nullable {
                    l.push_str(" NOT NULL");
                }
                if let Some(def) = c.default.as_deref().filter(|s| !s.is_empty()) {
                    l.push_str(&format!(" DEFAULT {def}"));
                }
                l
            })
            .collect();
        if !pks.is_empty() {
            let cols = pks
                .iter()
                .map(|p| ddl::ident(d, p))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("  PRIMARY KEY ({cols})"));
        }
        Ok(format!(
            "CREATE TABLE {} (\n{}\n);",
            ddl::qualify(d, schema, table),
            lines.join(",\n")
        ))
    }
}
