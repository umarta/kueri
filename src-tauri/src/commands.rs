use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use crate::db::connect::ConnectionConfigV2;
use crate::db::ddl::ColumnDef;
use crate::db::driver::{
    ColumnInfo, ForeignKey, IndexInfo, ProcessInfo, QueryResult, RoleInfo, SchemaInfo, TableInfo,
};
use crate::db::pool::AppState;
use crate::error::{AppError, AppResult};
use crate::sql_classify::{classify, SqlEffect};
use crate::ssh::profile::{SshProfile, SshRef};
use crate::ssh::store::SshProfileStore;

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
pub async fn connect(
    state: State<'_, AppState>,
    app: AppHandle,
    config: ConnectionConfigV2,
) -> AppResult<String> {
    let mut config = config;
    // Open an SSH tunnel first and point the driver at the local forward.
    let tunnel = if config.ssh.is_some() {
        let (local_port, child) = crate::db::tunnel::open(&app, &config).await?;
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
pub async fn list_databases(state: State<'_, AppState>, id: String) -> AppResult<Vec<String>> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    let driver = state.get(uuid)?;
    driver.list_databases().await
}

#[tauri::command]
pub async fn switch_database(
    state: State<'_, AppState>,
    app: AppHandle,
    id: String,
    database: String,
) -> AppResult<()> {
    let uuid =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid connection id: {e}")))?;
    let configs = crate::persist::load_connections(app.clone())?;
    let mut cfg = configs
        .iter()
        .find(|c| c.id == uuid)
        .cloned()
        .ok_or_else(|| {
            AppError::Other(format!("connection {id} not found in persisted configs"))
        })?;
    if cfg.ssh.is_some() {
        return Err(AppError::Other(
            "Switching databases on SSH-tunneled connections isn't supported yet — reconnect manually with the new database.".into(),
        ));
    }
    cfg.database = database;
    state.remove(uuid);
    let driver = crate::db::open(&cfg).await?;
    state.insert(uuid, Arc::from(driver));
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

// ── SSH profile commands ───────────────────────────────────────────────────────

/// Validation guard — rejects Phase 4-out-of-scope profile shapes.
pub(crate) fn validate_profile_for_save(profile: &SshProfile) -> AppResult<()> {
    if profile.jump.is_some() {
        return Err(AppError::Other(
            "SSH profile chains not supported in this release".into(),
        ));
    }
    Ok(())
}

/// Returns names of connections that reference the given profile via `SshRef::Profile`.
pub(crate) fn dependent_names_of(
    connections: &[ConnectionConfigV2],
    profile_id: Uuid,
) -> Vec<String> {
    connections
        .iter()
        .filter_map(|c| match &c.ssh {
            Some(SshRef::Profile(id)) if *id == profile_id => Some(c.name.clone()),
            _ => None,
        })
        .collect()
}

fn ssh_store(app: &AppHandle) -> AppResult<SshProfileStore> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Other(format!("config dir: {e}")))?;
    Ok(SshProfileStore::new(&dir))
}

#[tauri::command]
pub fn list_ssh_profiles(app: AppHandle) -> AppResult<Vec<SshProfile>> {
    ssh_store(&app)?.load()
}

#[tauri::command]
pub fn save_ssh_profile(app: AppHandle, profile: SshProfile) -> AppResult<()> {
    validate_profile_for_save(&profile)?;
    let store = ssh_store(&app)?;
    let mut profiles = store.load()?;
    match profiles.iter_mut().find(|p| p.id == profile.id) {
        Some(existing) => *existing = profile,
        None => profiles.push(profile),
    }
    store.save(&profiles)
}

#[tauri::command]
pub fn list_ssh_profile_dependents(app: AppHandle, id: String) -> AppResult<Vec<String>> {
    let profile_id =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid profile id: {e}")))?;
    let connections = crate::persist::load_connections(app.clone())?;
    Ok(dependent_names_of(&connections, profile_id))
}

#[tauri::command]
pub fn delete_ssh_profile(app: AppHandle, id: String) -> AppResult<()> {
    let profile_id =
        Uuid::parse_str(&id).map_err(|e| AppError::Other(format!("invalid profile id: {e}")))?;
    // Server-side re-check.
    let connections = crate::persist::load_connections(app.clone())?;
    let dependents = dependent_names_of(&connections, profile_id);
    if !dependents.is_empty() {
        return Err(AppError::Other(format!(
            "cannot delete SSH profile — in use by: {}",
            dependents.join(", ")
        )));
    }
    let store = ssh_store(&app)?;
    let mut profiles = store.load()?;
    profiles.retain(|p| p.id != profile_id);
    store.save(&profiles)
}

#[cfg(test)]
mod ssh_command_tests {
    use super::*;
    use uuid::Uuid;

    use crate::ssh::profile::{SshAuth, SshProfile};

    fn agent_profile() -> SshProfile {
        SshProfile {
            id: Uuid::new_v4(),
            name: "bastion".into(),
            host: "10.0.1.4".into(),
            port: 22,
            user: "ubuntu".into(),
            auth: SshAuth::Agent,
            jump: None,
        }
    }

    /// `save_ssh_profile`'s inner validator — pull the validation logic out
    /// into a testable helper so we can exercise it without a Tauri AppHandle.
    /// (The command wraps this helper + the store.save call.)
    #[test]
    fn validate_profile_rejects_jump_some() {
        let mut p = agent_profile();
        p.jump = Some(Uuid::new_v4());
        let err = validate_profile_for_save(&p).unwrap_err();
        assert!(
            err.to_string().contains("chains not supported"),
            "got: {err}"
        );
    }

    #[test]
    fn validate_profile_accepts_jump_none() {
        let p = agent_profile();
        assert!(validate_profile_for_save(&p).is_ok());
    }

    /// `dependent_names_of` — testable helper that walks a Vec<ConnectionConfigV2>
    /// looking for `SshRef::Profile(id)` matches. Returns names of dependents.
    #[test]
    fn dependent_names_finds_matching_profile_ref() {
        use crate::db::connect::ConnectionConfigV2;
        use crate::db::DbKind;
        use crate::safety::SafetyLevel;
        use crate::secrets::PasswordSource;
        use crate::ssh::profile::SshRef;

        let profile_id = Uuid::new_v4();
        let dependent = ConnectionConfigV2 {
            id: Uuid::new_v4(),
            schema_version: 2,
            name: "prod-analytics".into(),
            kind: DbKind::Postgres,
            host: "db".into(),
            port: 5432,
            database: "app".into(),
            user: "u".into(),
            password: PasswordSource::Keychain,
            tls: None,
            ssh: Some(SshRef::Profile(profile_id)),
            safety: SafetyLevel::default(),
            color: None,
            tags: vec![],
            file_path: None,
        };
        let names = dependent_names_of(&[dependent], profile_id);
        assert_eq!(names, vec!["prod-analytics".to_string()]);
    }

    #[test]
    fn dependent_names_ignores_inline_and_none_and_other_profile() {
        use crate::db::connect::ConnectionConfigV2;
        use crate::db::DbKind;
        use crate::safety::SafetyLevel;
        use crate::secrets::PasswordSource;
        use crate::ssh::profile::{SshAuth, SshProfile, SshRef};
        use std::path::PathBuf;

        let target_id = Uuid::new_v4();
        let other_profile_id = Uuid::new_v4();

        fn base() -> ConnectionConfigV2 {
            ConnectionConfigV2 {
                id: Uuid::new_v4(),
                schema_version: 2,
                name: "conn".into(),
                kind: DbKind::Postgres,
                host: "db".into(),
                port: 5432,
                database: "app".into(),
                user: "u".into(),
                password: PasswordSource::Keychain,
                tls: None,
                ssh: None,
                safety: SafetyLevel::default(),
                color: None,
                tags: vec![],
                file_path: None,
            }
        }

        let c_none = base();
        let c_inline = ConnectionConfigV2 {
            ssh: Some(SshRef::Inline(SshProfile {
                id: Uuid::new_v4(),
                name: "inline".into(),
                host: "".into(),
                port: 22,
                user: "".into(),
                auth: SshAuth::KeyFile {
                    path: PathBuf::from("/tmp/k"),
                    passphrase: None,
                },
                jump: None,
            })),
            ..base()
        };
        let c_other_profile = ConnectionConfigV2 {
            ssh: Some(SshRef::Profile(other_profile_id)),
            ..base()
        };

        let names = dependent_names_of(&[c_none, c_inline, c_other_profile], target_id);
        assert!(names.is_empty());
    }
}
