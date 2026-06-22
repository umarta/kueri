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
}

#[derive(Serialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub row_count: usize,
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

    // ── DDL ──────────────────────────────────────────────────────────────────
    // Each driver reports its SQL dialect; the default methods below build the
    // right DDL and run it. This keeps every database difference in `ddl.rs`,
    // so the UI just calls these (never branching on database type).

    /// The SQL dialect this driver speaks (drives DDL generation).
    fn dialect(&self) -> Dialect;

    async fn create_table(&self, schema: &str, name: &str, columns: &[ColumnDef]) -> AppResult<()> {
        self.run_query(&ddl::create_table(self.dialect(), schema, name, columns)).await?;
        Ok(())
    }
    async fn drop_table(&self, schema: &str, table: &str) -> AppResult<()> {
        self.run_query(&ddl::drop_table(self.dialect(), schema, table)).await?;
        Ok(())
    }
    async fn rename_table(&self, schema: &str, old: &str, new: &str) -> AppResult<()> {
        self.run_query(&ddl::rename_table(self.dialect(), schema, old, new)).await?;
        Ok(())
    }
    async fn truncate_table(&self, schema: &str, table: &str) -> AppResult<()> {
        self.run_query(&ddl::truncate_table(self.dialect(), schema, table)).await?;
        Ok(())
    }
    async fn duplicate_table(&self, schema: &str, src: &str, dst: &str) -> AppResult<()> {
        self.run_query(&ddl::duplicate_table(self.dialect(), schema, src, dst)).await?;
        Ok(())
    }
    async fn add_column(&self, schema: &str, table: &str, column: &ColumnDef) -> AppResult<()> {
        self.run_query(&ddl::add_column(self.dialect(), schema, table, column)).await?;
        Ok(())
    }
    async fn drop_column(&self, schema: &str, table: &str, column: &str) -> AppResult<()> {
        self.run_query(&ddl::drop_column(self.dialect(), schema, table, column)).await?;
        Ok(())
    }
    async fn rename_column(&self, schema: &str, table: &str, old: &str, new: &str) -> AppResult<()> {
        self.run_query(&ddl::rename_column(self.dialect(), schema, table, old, new)).await?;
        Ok(())
    }
    async fn change_column_type(
        &self, schema: &str, table: &str, column: &str, new_type: &str, not_null: bool,
    ) -> AppResult<()> {
        if self.dialect() == Dialect::Sqlite {
            return Err(AppError::Other(
                "SQLite can't change a column type in place (requires a table rebuild).".into(),
            ));
        }
        self.run_query(&ddl::change_column_type(self.dialect(), schema, table, column, new_type, not_null)).await?;
        Ok(())
    }
    async fn set_column_nullable(
        &self, schema: &str, table: &str, column: &str, current_type: &str, not_null: bool,
    ) -> AppResult<()> {
        if self.dialect() == Dialect::Sqlite {
            return Err(AppError::Other(
                "SQLite can't change a column's nullability in place (requires a table rebuild).".into(),
            ));
        }
        self.run_query(&ddl::set_column_nullable(self.dialect(), schema, table, column, current_type, not_null)).await?;
        Ok(())
    }
}
