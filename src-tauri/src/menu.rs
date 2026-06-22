//! Native application menu. Items emit a `"menu"` event (their id) to the
//! frontend, which maps ids to actions. Accelerators here are the single source
//! of truth for global commands; context-sensitive keys (Space, ⌘F, ⌘1–9,
//! ⌘⌃[/]) stay in the frontend so the editor's own shortcuts keep working.

use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{App, Emitter};

fn item(app: &App, id: &str, label: &str, accel: Option<&str>) -> tauri::Result<tauri::menu::MenuItem<tauri::Wry>> {
    let mut b = MenuItemBuilder::with_id(id, label);
    if let Some(a) = accel {
        b = b.accelerator(a);
    }
    b.build(app)
}

pub fn build(app: &App) -> tauri::Result<()> {
    let app_menu = SubmenuBuilder::new(app, "Kueri")
        .about(None)
        .separator()
        .item(&item(app, "settings", "Settings…", Some("CmdOrCtrl+Comma"))?)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&item(app, "new_query_tab", "New Query Tab", Some("CmdOrCtrl+T"))?)
        .item(&item(app, "new_sql", "New SQL Editor", Some("CmdOrCtrl+E"))?)
        .item(&item(app, "new_table", "New Table…", None)?)
        .separator()
        .item(&item(app, "close_tab", "Close Tab", Some("CmdOrCtrl+W"))?)
        .build()?;

    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .separator()
        .item(&item(app, "commit", "Commit Changes", Some("CmdOrCtrl+S"))?)
        .item(&item(app, "add_row", "Add Row", Some("CmdOrCtrl+I"))?)
        .build()?;

    let view_menu = SubmenuBuilder::new(app, "View")
        .item(&item(app, "data_view", "Data", None)?)
        .item(&item(app, "structure_view", "Structure", None)?)
        .separator()
        .item(&item(app, "toggle_sidebar", "Toggle Sidebar", None)?)
        .item(&item(app, "toggle_detail", "Toggle Row Detail", None)?)
        .item(&item(app, "toggle_log", "Toggle Query Log", None)?)
        .separator()
        .item(&item(app, "open_palette", "Open Anything…", Some("CmdOrCtrl+P"))?)
        .separator()
        .item(&item(app, "force_reload", "Force Reload", Some("CmdOrCtrl+Shift+R"))?)
        .build()?;

    let conn_menu = SubmenuBuilder::new(app, "Connection")
        .item(&item(app, "new_connection", "New Connection…", Some("CmdOrCtrl+N"))?)
        .item(&item(app, "switch_schema", "Switch Schema", Some("CmdOrCtrl+K"))?)
        .separator()
        .item(&item(app, "run_query", "Run Query", None)?)
        .separator()
        .item(&item(app, "refresh", "Reload", Some("CmdOrCtrl+R"))?)
        .item(&item(app, "disconnect", "Disconnect", None)?)
        .build()?;

    let window_menu = SubmenuBuilder::new(app, "Window")
        .item(&item(app, "prev_tab", "Previous Tab", Some("CmdOrCtrl+BracketLeft"))?)
        .item(&item(app, "next_tab", "Next Tab", Some("CmdOrCtrl+BracketRight"))?)
        .separator()
        .minimize()
        .maximize()
        .build()?;

    let menu = MenuBuilder::new(app)
        .items(&[&app_menu, &file_menu, &edit_menu, &view_menu, &conn_menu, &window_menu])
        .build()?;

    app.set_menu(menu)?;
    app.on_menu_event(move |app, event| {
        let _ = app.emit("menu", event.id().0.as_str());
    });
    Ok(())
}
