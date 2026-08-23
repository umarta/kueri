//! Query history persistence: reads/writes history.json in app_config_dir.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    pub time: String,
    pub date: String,
    pub sql: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_count: Option<u64>,
    pub connection_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub struct HistoryStore {
    file_path: PathBuf,
}

impl HistoryStore {
    pub fn new(config_dir: &Path) -> Self {
        Self {
            file_path: config_dir.join("history.json"),
        }
    }

    /// Returns `[]` if the file is missing. Propagates JSON parse errors.
    pub fn load(&self) -> AppResult<Vec<HistoryEntry>> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }
        let bytes = fs::read(&self.file_path)
            .map_err(|e| AppError::Other(format!("read history.json: {e}")))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| AppError::Other(format!("parse history.json: {e}")))
    }

    /// Atomic write: serialise → `history.json.tmp` → rename over `history.json`.
    pub fn save(&self, entries: &[HistoryEntry]) -> AppResult<()> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| AppError::Other(format!("mkdir {}: {e}", parent.display())))?;
        }
        let json = serde_json::to_vec_pretty(entries)
            .map_err(|e| AppError::Other(format!("serialize history.json: {e}")))?;
        let tmp = self.file_path.with_extension("json.tmp");
        fs::write(&tmp, &json)
            .map_err(|e| AppError::Other(format!("write {}: {e}", tmp.display())))?;
        fs::rename(&tmp, &self.file_path).map_err(|e| {
            AppError::Other(format!(
                "rename {} → {}: {e}",
                tmp.display(),
                self.file_path.display()
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_entries() -> Vec<HistoryEntry> {
        vec![
            HistoryEntry {
                id: 1,
                time: "10:00:00".into(),
                date: "2026-08-23".into(),
                sql: "SELECT 1".into(),
                ms: Some(5),
                row_count: Some(1),
                connection_id: Some("conn-abc".into()),
                error: None,
            },
            HistoryEntry {
                id: 2,
                time: "10:01:00".into(),
                date: "2026-08-23".into(),
                sql: "DROP TABLE oops".into(),
                ms: Some(3),
                row_count: None,
                connection_id: Some("conn-abc".into()),
                error: Some("permission denied".into()),
            },
        ]
    }

    #[test]
    fn load_missing_file_returns_empty_vec() {
        let dir = TempDir::new().unwrap();
        let store = HistoryStore::new(dir.path());
        let entries = store.load().unwrap();
        assert!(entries.is_empty());
        assert!(!dir.path().join("history.json").exists());
    }

    #[test]
    fn save_then_load_round_trips_all_fields() {
        let dir = TempDir::new().unwrap();
        let store = HistoryStore::new(dir.path());
        let want = sample_entries();
        store.save(&want).unwrap();
        let got = store.load().unwrap();
        assert_eq!(got, want);
    }

    #[test]
    fn save_leaves_no_tmp_sibling() {
        let dir = TempDir::new().unwrap();
        let store = HistoryStore::new(dir.path());
        store.save(&sample_entries()).unwrap();
        assert!(dir.path().join("history.json").exists());
        assert!(!dir.path().join("history.json.tmp").exists());
    }
}
