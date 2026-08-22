//! Cross-launch workspace snapshot: persists per-connection tab/schema state
//! to `<config_dir>/workspaces.json` with atomic writes.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

pub const WORKSPACE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PersistedTab {
    Query {
        id: String,
        title: String,
        sql: String,
    },
    Table {
        id: String,
        schema: String,
        table: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedWorkspace {
    pub connection_id: Uuid,
    pub active_schema: String,
    pub focused_tab_id: Option<String>,
    pub tabs: Vec<PersistedTab>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceFile {
    pub schema_version: u32,
    pub last_active_id: Option<Uuid>,
    pub workspaces: Vec<PersistedWorkspace>,
}

impl WorkspaceFile {
    /// Empty snapshot at the current schema version.
    pub fn empty() -> Self {
        Self {
            schema_version: WORKSPACE_SCHEMA_VERSION,
            last_active_id: None,
            workspaces: Vec::new(),
        }
    }
}

pub struct WorkspaceStore {
    file_path: PathBuf,
}

impl WorkspaceStore {
    pub fn new(config_dir: &Path) -> Self {
        Self {
            file_path: config_dir.join("workspaces.json"),
        }
    }

    /// Read `workspaces.json`. Missing file returns an empty v1 snapshot.
    /// Unknown `schema_version` is a hard error (caller may log and treat as empty).
    pub fn load(&self) -> AppResult<WorkspaceFile> {
        if !self.file_path.exists() {
            return Ok(WorkspaceFile::empty());
        }
        let bytes = fs::read(&self.file_path)
            .map_err(|e| AppError::Other(format!("read workspaces.json: {e}")))?;
        let file: WorkspaceFile = serde_json::from_slice(&bytes)
            .map_err(|e| AppError::Other(format!("parse workspaces.json: {e}")))?;
        if file.schema_version != WORKSPACE_SCHEMA_VERSION {
            return Err(AppError::Other(format!(
                "workspaces.json schema version {} not supported (expected {WORKSPACE_SCHEMA_VERSION})",
                file.schema_version
            )));
        }
        Ok(file)
    }

    /// Write atomically: serialise to `<path>.tmp`, then rename over `<path>`.
    /// A crash mid-write leaves either the old file or the new file, never a half-write.
    pub fn save(&self, file: &WorkspaceFile) -> AppResult<()> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| AppError::Other(format!("mkdir {}: {e}", parent.display())))?;
        }
        let json = serde_json::to_vec_pretty(file)
            .map_err(|e| AppError::Other(format!("serialize workspaces.json: {e}")))?;
        let tmp = self.file_path.with_extension("json.tmp");
        fs::write(&tmp, &json)
            .map_err(|e| AppError::Other(format!("write {}: {e}", tmp.display())))?;
        fs::rename(&tmp, &self.file_path).map_err(|e| {
            AppError::Other(format!(
                "rename {} → {}: {e}",
                tmp.display(),
                self.file_path.display()
            ))
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_file() -> WorkspaceFile {
        WorkspaceFile {
            schema_version: WORKSPACE_SCHEMA_VERSION,
            last_active_id: Some(Uuid::from_u128(42)),
            workspaces: vec![PersistedWorkspace {
                connection_id: Uuid::from_u128(42),
                active_schema: "kame".into(),
                focused_tab_id: Some("tab-3".into()),
                tabs: vec![
                    PersistedTab::Query {
                        id: "tab-1".into(),
                        title: "Untitled".into(),
                        sql: "SELECT * FROM users;".into(),
                    },
                    PersistedTab::Table {
                        id: "tab-3".into(),
                        schema: "kame".into(),
                        table: "banks".into(),
                    },
                ],
            }],
        }
    }

    #[test]
    fn load_missing_file_returns_empty_v1() {
        let dir = TempDir::new().unwrap();
        let store = WorkspaceStore::new(dir.path());
        let file = store.load().unwrap();
        assert_eq!(file.schema_version, WORKSPACE_SCHEMA_VERSION);
        assert!(file.workspaces.is_empty());
        assert!(file.last_active_id.is_none());
        assert!(!dir.path().join("workspaces.json").exists());
    }

    #[test]
    fn save_then_load_round_trips_all_fields() {
        let dir = TempDir::new().unwrap();
        let store = WorkspaceStore::new(dir.path());
        let want = sample_file();
        store.save(&want).unwrap();
        let got = store.load().unwrap();
        assert_eq!(got, want);
    }

    #[test]
    fn load_rejects_unknown_schema_version() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("workspaces.json");
        fs::write(
            &path,
            br#"{"schema_version":999,"last_active_id":null,"workspaces":[]}"#,
        )
        .unwrap();
        let store = WorkspaceStore::new(dir.path());
        let err = store.load().unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("schema version 999"));
    }

    #[test]
    fn save_leaves_no_tmp_sibling() {
        let dir = TempDir::new().unwrap();
        let store = WorkspaceStore::new(dir.path());
        store.save(&sample_file()).unwrap();
        assert!(dir.path().join("workspaces.json").exists());
        assert!(!dir.path().join("workspaces.json.tmp").exists());
    }
}
