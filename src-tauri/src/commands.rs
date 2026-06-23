use std::sync::Arc;
use tauri::State;

use crate::db::connect::ConnectionConfig;
use crate::db::ddl::ColumnDef;
use crate::db::driver::{ColumnInfo, ForeignKey, IndexInfo, QueryResult, SchemaInfo, TableInfo};
use crate::db::pool::AppState;
use crate::error::{AppError, AppResult};

/// Write text to a path (used by CSV/JSON export; the path comes from a save dialog).
#[tauri::command]
pub fn write_text_file(path: String, content: String) -> AppResult<()> {
    std::fs::write(&path, content).map_err(|e| AppError::Other(format!("write {path}: {e}")))
}

#[tauri::command]
pub async fn connect(state: State<'_, AppState>, config: ConnectionConfig) -> AppResult<String> {
    let driver = crate::db::open(&config).await?;
    state.insert(config.id.clone(), Arc::from(driver));
    Ok(config.id)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.remove(&id);
    Ok(())
}

#[tauri::command]
pub async fn list_schemas(state: State<'_, AppState>, id: String) -> AppResult<Vec<SchemaInfo>> {
    state.get(&id)?.list_schemas().await
}

#[tauri::command]
pub async fn list_tables(
    state: State<'_, AppState>,
    id: String,
    schema: String,
) -> AppResult<Vec<TableInfo>> {
    state.get(&id)?.list_tables(&schema).await
}

#[tauri::command]
pub async fn list_columns(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
) -> AppResult<Vec<ColumnInfo>> {
    state.get(&id)?.list_columns(&schema, &table).await
}

#[tauri::command]
pub async fn table_ddl(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
) -> AppResult<String> {
    state.get(&id)?.table_ddl(&schema, &table).await
}

#[tauri::command]
pub async fn foreign_keys(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
) -> AppResult<Vec<ForeignKey>> {
    state.get(&id)?.list_foreign_keys(&schema, &table).await
}

#[tauri::command]
pub async fn list_indexes(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
) -> AppResult<Vec<IndexInfo>> {
    state.get(&id)?.list_indexes(&schema, &table).await
}

#[tauri::command]
pub async fn create_index(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
    name: String,
    columns: Vec<String>,
    unique: bool,
) -> AppResult<()> {
    state
        .get(&id)?
        .create_index(&schema, &table, &name, &columns, unique)
        .await
}

#[tauri::command]
pub async fn drop_index(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
    name: String,
) -> AppResult<()> {
    state.get(&id)?.drop_index(&schema, &table, &name).await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn add_foreign_key(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
    column: String,
    ref_table: String,
    ref_column: String,
    name: String,
    validate: bool,
) -> AppResult<()> {
    state
        .get(&id)?
        .add_foreign_key(
            &schema,
            &table,
            &column,
            &ref_table,
            &ref_column,
            &name,
            validate,
        )
        .await
}

#[tauri::command]
pub async fn primary_keys(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
) -> AppResult<Vec<String>> {
    state.get(&id)?.list_primary_keys(&schema, &table).await
}

#[tauri::command]
pub async fn execute_query(
    state: State<'_, AppState>,
    id: String,
    sql: String,
    query_id: String,
) -> AppResult<QueryResult> {
    let driver = state.get(&id)?;
    // Run on a task we can abort, so `cancel_query` can stop a long-running query.
    let task = tokio::spawn(async move { driver.run_query(&sql).await });
    state.register_query(query_id.clone(), task.abort_handle());
    let res = task.await;
    state.finish_query(&query_id);
    match res {
        Ok(inner) => inner,
        Err(e) if e.is_cancelled() => Err(AppError::Other("Query cancelled.".into())),
        Err(e) => Err(AppError::Other(format!("query task failed: {e}"))),
    }
}

#[tauri::command]
pub fn cancel_query(state: State<'_, AppState>, query_id: String) {
    state.cancel(&query_id);
}

// ── DDL commands (database-agnostic; the driver renders the right SQL) ─────────

#[tauri::command]
pub async fn create_table(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    name: String,
    columns: Vec<ColumnDef>,
) -> AppResult<()> {
    state.get(&id)?.create_table(&schema, &name, &columns).await
}

#[tauri::command]
pub async fn drop_table(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
) -> AppResult<()> {
    state.get(&id)?.drop_table(&schema, &table).await
}

#[tauri::command]
pub async fn rename_table(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    old: String,
    new: String,
) -> AppResult<()> {
    state.get(&id)?.rename_table(&schema, &old, &new).await
}

#[tauri::command]
pub async fn truncate_table(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
) -> AppResult<()> {
    state.get(&id)?.truncate_table(&schema, &table).await
}

#[tauri::command]
pub async fn duplicate_table(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    src: String,
    dst: String,
) -> AppResult<()> {
    state.get(&id)?.duplicate_table(&schema, &src, &dst).await
}

#[tauri::command]
pub async fn add_column(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
    column: ColumnDef,
) -> AppResult<()> {
    state.get(&id)?.add_column(&schema, &table, &column).await
}

#[tauri::command]
pub async fn drop_column(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
    column: String,
) -> AppResult<()> {
    state.get(&id)?.drop_column(&schema, &table, &column).await
}

#[tauri::command]
pub async fn rename_column(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
    old: String,
    new: String,
) -> AppResult<()> {
    state
        .get(&id)?
        .rename_column(&schema, &table, &old, &new)
        .await
}

#[tauri::command]
pub async fn change_column_type(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
    column: String,
    new_type: String,
    not_null: bool,
) -> AppResult<()> {
    state
        .get(&id)?
        .change_column_type(&schema, &table, &column, &new_type, not_null)
        .await
}

#[tauri::command]
pub async fn set_column_nullable(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
    column: String,
    current_type: String,
    not_null: bool,
) -> AppResult<()> {
    state
        .get(&id)?
        .set_column_nullable(&schema, &table, &column, &current_type, not_null)
        .await
}
