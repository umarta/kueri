//! Connection persistence + password keychain.
//!
//! Connections (without passwords) are stored as JSON in the app config dir.
//! Passwords live in the OS keychain, keyed by connection id — never on disk.
//! Both are plain custom commands, so no plugin/ACL wiring is needed.

use std::fs;
use std::path::PathBuf;

use keyring::Entry;
use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};

const KEYRING_SERVICE: &str = "dev.kueri.app";

fn connections_path(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Other(format!("config dir: {e}")))?;
    fs::create_dir_all(&dir).map_err(|e| AppError::Other(e.to_string()))?;
    Ok(dir.join("connections.json"))
}

#[tauri::command]
pub fn load_connections(app: AppHandle) -> AppResult<Vec<Value>> {
    let path = connections_path(&app)?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = fs::read_to_string(&path).map_err(|e| AppError::Other(e.to_string()))?;
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

#[tauri::command]
pub fn save_connections(app: AppHandle, connections: Vec<Value>) -> AppResult<()> {
    let path = connections_path(&app)?;
    let raw =
        serde_json::to_string_pretty(&connections).map_err(|e| AppError::Other(e.to_string()))?;
    fs::write(&path, raw).map_err(|e| AppError::Other(e.to_string()))
}

fn entry(id: &str) -> AppResult<Entry> {
    Entry::new(KEYRING_SERVICE, id).map_err(|e| AppError::Other(format!("keychain: {e}")))
}

#[tauri::command]
pub fn secret_set(id: String, password: String) -> AppResult<()> {
    entry(&id)?
        .set_password(&password)
        .map_err(|e| AppError::Other(format!("keychain: {e}")))
}

#[tauri::command]
pub fn secret_get(id: String) -> AppResult<Option<String>> {
    match entry(&id)?.get_password() {
        Ok(p) => Ok(Some(p)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Other(format!("keychain: {e}"))),
    }
}

#[tauri::command]
pub fn secret_delete(id: String) -> AppResult<()> {
    match entry(&id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Other(format!("keychain: {e}"))),
    }
}
