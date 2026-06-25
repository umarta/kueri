use std::sync::Arc;
use tauri::State;

use crate::db::connect::ConnectionConfig;
use crate::db::ddl::ColumnDef;
use crate::db::driver::{
    ColumnInfo, ForeignKey, IndexInfo, ProcessInfo, QueryResult, RoleInfo, SchemaInfo, TableInfo,
};
use crate::db::pool::AppState;
use crate::error::{AppError, AppResult};

/// Write text to a path (used by CSV/JSON export; the path comes from a save dialog).
#[tauri::command]
pub fn write_text_file(path: String, content: String) -> AppResult<()> {
    std::fs::write(&path, content).map_err(|e| AppError::Other(format!("write {path}: {e}")))
}

/// Read a text file (CSV import; the path comes from an open dialog).
#[tauri::command]
pub fn read_text_file(path: String) -> AppResult<String> {
    std::fs::read_to_string(&path).map_err(|e| AppError::Other(format!("read {path}: {e}")))
}

#[tauri::command]
pub async fn connect(state: State<'_, AppState>, config: ConnectionConfig) -> AppResult<String> {
    let mut config = config;
    // Open an SSH tunnel first and point the driver at the local forward.
    let tunnel = if config.ssh_enabled {
        let (local_port, child) = crate::db::tunnel::open(&config).await?;
        config.host = "127.0.0.1".into();
        config.port = local_port;
        Some(child)
    } else {
        None
    };
    // If db::open fails, `tunnel` drops here and kill_on_drop tears it down.
    let driver = crate::db::open(&config).await?;
    state.insert(config.id.clone(), Arc::from(driver));
    if let Some(child) = tunnel {
        state.insert_tunnel(config.id.clone(), child);
    }
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
pub async fn view_definition(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    name: String,
) -> AppResult<String> {
    state.get(&id)?.view_definition(&schema, &name).await
}

#[tauri::command]
pub async fn list_objects(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    kind: String,
) -> AppResult<Vec<String>> {
    state.get(&id)?.list_objects(&schema, &kind).await
}

#[tauri::command]
pub async fn object_definition(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    name: String,
    kind: String,
) -> AppResult<String> {
    state
        .get(&id)?
        .object_definition(&schema, &name, &kind)
        .await
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

#[tauri::command]
pub async fn begin_txn(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.get(&id)?.begin().await
}

#[tauri::command]
pub async fn commit_txn(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.get(&id)?.commit().await
}

#[tauri::command]
pub async fn rollback_txn(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.get(&id)?.rollback().await
}

#[tauri::command]
pub async fn list_processes(state: State<'_, AppState>, id: String) -> AppResult<Vec<ProcessInfo>> {
    state.get(&id)?.list_processes().await
}

#[tauri::command]
pub async fn kill_process(state: State<'_, AppState>, id: String, pid: String) -> AppResult<()> {
    state.get(&id)?.kill_process(&pid).await
}

#[tauri::command]
pub async fn list_roles(state: State<'_, AppState>, id: String) -> AppResult<Vec<RoleInfo>> {
    state.get(&id)?.list_roles().await
}

#[tauri::command]
pub async fn set_column_comment(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
    column: String,
    comment: String,
) -> AppResult<()> {
    state
        .get(&id)?
        .set_column_comment(&schema, &table, &column, &comment)
        .await
}

#[tauri::command]
pub async fn create_schema(state: State<'_, AppState>, id: String, name: String) -> AppResult<()> {
    state.get(&id)?.create_schema(&name).await
}

#[tauri::command]
pub async fn drop_schema(state: State<'_, AppState>, id: String, name: String) -> AppResult<()> {
    state.get(&id)?.drop_schema(&name).await
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
