use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

use crate::db::connect::ConnectionConfigV2;
use crate::db::ddl::ColumnDef;
use crate::db::driver::{
    ColumnInfo, ForeignKey, IndexInfo, ProcessInfo, QueryResult, RoleInfo, SchemaInfo, TableInfo,
};
use crate::db::pool::AppState;
use crate::error::{AppError, AppResult};
use crate::sql_classify::{classify, SqlEffect};

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
pub async fn connect(state: State<'_, AppState>, config: ConnectionConfigV2) -> AppResult<String> {
    let mut config = config;
    // Open an SSH tunnel first and point the driver at the local forward.
    let tunnel = if config.ssh.is_some() {
        let (local_port, child) = crate::db::tunnel::open(&config).await?;
        config.host = "127.0.0.1".into();
        config.port = local_port;
        Some(child)
    } else {
        None
    };
    // If db::open fails, `tunnel` drops here and kill_on_drop tears it down.
    let driver = crate::db::open(&config).await?;
    let id_uuid = config.id;
    let id_str = id_uuid.to_string();
    state.insert(id_uuid, Arc::from(driver));
    if let Some(child) = tunnel {
        state.insert_tunnel(id_uuid, child);
    }
    Ok(id_str)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.remove(uuid);
    Ok(())
}

#[tauri::command]
pub async fn list_schemas(state: State<'_, AppState>, id: String) -> AppResult<Vec<SchemaInfo>> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    let driver = state.get(uuid)?;
    state.schema_cache.schemas(uuid, driver.as_ref()).await
}

#[tauri::command]
pub async fn list_tables(
    state: State<'_, AppState>,
    id: String,
    schema: String,
) -> AppResult<Vec<TableInfo>> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    let driver = state.get(uuid)?;
    state
        .schema_cache
        .tables(uuid, &schema, driver.as_ref())
        .await
}

#[tauri::command]
pub async fn list_columns(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
) -> AppResult<Vec<ColumnInfo>> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    let driver = state.get(uuid)?;
    state
        .schema_cache
        .columns(uuid, &schema, &table, driver.as_ref())
        .await
}

#[tauri::command]
pub async fn table_ddl(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
) -> AppResult<String> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.table_ddl(&schema, &table).await
}

#[tauri::command]
pub async fn view_definition(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    name: String,
) -> AppResult<String> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.view_definition(&schema, &name).await
}

#[tauri::command]
pub async fn list_objects(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    kind: String,
) -> AppResult<Vec<String>> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.list_objects(&schema, &kind).await
}

#[tauri::command]
pub async fn object_definition(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    name: String,
    kind: String,
) -> AppResult<String> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state
        .get(uuid)?
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
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.list_foreign_keys(&schema, &table).await
}

#[tauri::command]
pub async fn list_indexes(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
) -> AppResult<Vec<IndexInfo>> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.list_indexes(&schema, &table).await
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
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state
        .get(uuid)?
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
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.drop_index(&schema, &table, &name).await
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
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state
        .get(uuid)?
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
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.list_primary_keys(&schema, &table).await
}

#[tauri::command]
pub async fn execute_query(
    state: State<'_, AppState>,
    id: String,
    sql: String,
    query_id: String,
    safety: crate::safety::SafetyLevel,
) -> AppResult<QueryResult> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;

    // Pre-flight: classify each statement, take the strictest decision.
    for stmt in crate::sql_classify::split_statements(&sql) {
        let effect = crate::sql_classify::classify_one(&stmt);
        let has_where = matches!(effect, crate::sql_classify::SqlEffect::Write)
            && crate::sql_classify::has_where_clause(&stmt);
        match safety.decide(effect, has_where, &stmt) {
            crate::safety::SafetyDecision::Allow => continue,
            crate::safety::SafetyDecision::NeedsConfirmation { reason, statement } => {
                let token = state.confirm_tokens.issue(uuid, sql.clone());
                return Err(AppError::NeedsConfirmation {
                    token,
                    statement,
                    reason,
                });
            }
            crate::safety::SafetyDecision::Reject { reason, statement } => {
                return Err(AppError::SafetyRejected { statement, reason });
            }
        }
    }

    // Existing execute path — unchanged from KUE-002 (schema-cache invalidation on DDL).
    let driver = state.get(uuid)?;
    // Classify the SQL to detect DDL statements.
    let effects = classify(&sql);
    // Run on a task we can abort, so `cancel_query` can stop a long-running query.
    let task = tokio::spawn(async move { driver.run_query(&sql).await });
    state.register_query(query_id.clone(), task.abort_handle());
    let res = task.await;
    state.finish_query(&query_id);
    // Invalidate schema cache if this was a DDL statement, whether or not the query succeeded.
    if effects.iter().any(|e| matches!(e, SqlEffect::Ddl)) {
        state.schema_cache.invalidate(uuid);
    }
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
pub async fn execute_query_confirmed(
    state: State<'_, AppState>,
    token: String,
    query_id: String,
) -> AppResult<QueryResult> {
    let (uuid, sql) = state.confirm_tokens.consume(&token)?;

    // No pre-flight — the token IS the authorization (issued by execute_query's classifier).
    let driver = state.get(uuid)?;
    let effects = crate::sql_classify::classify(&sql);
    // Run on a task we can abort, so `cancel_query` can stop a long-running query.
    let task = tokio::spawn(async move { driver.run_query(&sql).await });
    state.register_query(query_id.clone(), task.abort_handle());
    let res = task.await;
    state.finish_query(&query_id);
    // Invalidate schema cache if this was a DDL statement, whether or not the query succeeded.
    if effects
        .iter()
        .any(|e| matches!(e, crate::sql_classify::SqlEffect::Ddl))
    {
        state.schema_cache.invalidate(uuid);
    }
    match res {
        Ok(inner) => inner,
        Err(e) if e.is_cancelled() => Err(AppError::Other("Query cancelled.".into())),
        Err(e) => Err(AppError::Other(format!("query task failed: {e}"))),
    }
}

#[tauri::command]
pub async fn begin_txn(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.begin().await
}

#[tauri::command]
pub async fn commit_txn(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.commit().await
}

#[tauri::command]
pub async fn rollback_txn(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.rollback().await
}

#[tauri::command]
pub async fn list_processes(state: State<'_, AppState>, id: String) -> AppResult<Vec<ProcessInfo>> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.list_processes().await
}

#[tauri::command]
pub async fn kill_process(state: State<'_, AppState>, id: String, pid: String) -> AppResult<()> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.kill_process(&pid).await
}

#[tauri::command]
pub async fn list_roles(state: State<'_, AppState>, id: String) -> AppResult<Vec<RoleInfo>> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.list_roles().await
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
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state
        .get(uuid)?
        .set_column_comment(&schema, &table, &column, &comment)
        .await
}

#[tauri::command]
pub async fn create_schema(state: State<'_, AppState>, id: String, name: String) -> AppResult<()> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.create_schema(&name).await
}

#[tauri::command]
pub async fn drop_schema(state: State<'_, AppState>, id: String, name: String) -> AppResult<()> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.drop_schema(&name).await
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
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state
        .get(uuid)?
        .create_table(&schema, &name, &columns)
        .await
}

#[tauri::command]
pub async fn drop_table(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
) -> AppResult<()> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.drop_table(&schema, &table).await
}

#[tauri::command]
pub async fn rename_table(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    old: String,
    new: String,
) -> AppResult<()> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.rename_table(&schema, &old, &new).await
}

#[tauri::command]
pub async fn truncate_table(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
) -> AppResult<()> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.truncate_table(&schema, &table).await
}

#[tauri::command]
pub async fn duplicate_table(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    src: String,
    dst: String,
) -> AppResult<()> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.duplicate_table(&schema, &src, &dst).await
}

#[tauri::command]
pub async fn add_column(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
    column: ColumnDef,
) -> AppResult<()> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.add_column(&schema, &table, &column).await
}

#[tauri::command]
pub async fn drop_column(
    state: State<'_, AppState>,
    id: String,
    schema: String,
    table: String,
    column: String,
) -> AppResult<()> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.get(uuid)?.drop_column(&schema, &table, &column).await
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
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state
        .get(uuid)?
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
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state
        .get(uuid)?
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
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state
        .get(uuid)?
        .set_column_nullable(&schema, &table, &column, &current_type, not_null)
        .await
}

#[tauri::command]
pub fn refresh_schema(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    state.schema_cache.invalidate(uuid);
    Ok(())
}
