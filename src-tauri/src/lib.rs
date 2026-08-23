mod clients;
mod commands;
mod confirm_tokens;
pub mod db;
mod error;
mod menu;
mod migration;
mod persist;
mod pgtools;
pub mod safety;
mod schema_cache;
pub mod secrets;
mod sql_classify;
pub mod ssh {
    pub mod profile;
    pub mod store;
}
pub mod tls;
pub mod workspace_persist;

use db::pool::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // WebKitGTK's DMABUF renderer crashes the web process on a number of Linux
    // GPU/driver stacks (blank window → SIGABRT). Disabling it is the standard
    // fix and costs nothing elsewhere. Respect an explicit user override.
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            menu::build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connect,
            commands::disconnect,
            commands::list_databases,
            commands::switch_database,
            commands::list_schemas,
            commands::list_tables,
            commands::list_columns,
            commands::table_ddl,
            commands::view_definition,
            commands::list_objects,
            commands::object_definition,
            commands::foreign_keys,
            commands::list_indexes,
            commands::create_index,
            commands::drop_index,
            commands::add_foreign_key,
            commands::primary_keys,
            commands::execute_query,
            commands::execute_query_confirmed,
            commands::execute_query_params,
            commands::cancel_query,
            commands::begin_txn,
            commands::commit_txn,
            commands::rollback_txn,
            commands::list_processes,
            commands::kill_process,
            commands::list_roles,
            commands::set_column_comment,
            commands::create_schema,
            commands::drop_schema,
            commands::create_table,
            commands::drop_table,
            commands::rename_table,
            commands::truncate_table,
            commands::duplicate_table,
            commands::add_column,
            commands::drop_column,
            commands::rename_column,
            commands::change_column_type,
            commands::set_column_nullable,
            commands::refresh_schema,
            commands::write_text_file,
            commands::read_text_file,
            commands::list_ssh_profiles,
            commands::save_ssh_profile,
            commands::delete_ssh_profile,
            commands::list_ssh_profile_dependents,
            commands::load_workspaces,
            commands::save_workspaces,
            pgtools::pg_export,
            pgtools::pg_import,
            clients::detect_clients,
            clients::install_pg_client,
            clients::open_url,
            persist::load_connections,
            persist::save_connections,
            persist::secret_set,
            persist::secret_get,
            persist::secret_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Kueri");
}
