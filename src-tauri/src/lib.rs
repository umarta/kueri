mod commands;
mod db;
mod error;
mod menu;
mod persist;
mod pgtools;

use db::pool::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            commands::list_schemas,
            commands::list_tables,
            commands::list_columns,
            commands::primary_keys,
            commands::execute_query,
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
            commands::write_text_file,
            pgtools::pg_export,
            pgtools::pg_import,
            persist::load_connections,
            persist::save_connections,
            persist::secret_set,
            persist::secret_get,
            persist::secret_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Kueri");
}
