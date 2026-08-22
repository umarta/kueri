//! Persistence for SSH profiles — stateless store that reads/writes
//! `ssh_profiles.json` in the app config dir.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::ssh::profile::SshProfile;

pub const SSH_PROFILES_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct SshProfileFile {
    pub schema_version: u32,
    pub profiles: Vec<SshProfile>,
}

pub struct SshProfileStore {
    file_path: PathBuf,
}

impl SshProfileStore {
    pub fn new(config_dir: &Path) -> Self {
        Self {
            file_path: config_dir.join("ssh_profiles.json"),
        }
    }

    pub fn load(&self) -> AppResult<Vec<SshProfile>> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }
        let raw = std::fs::read_to_string(&self.file_path)
            .map_err(|e| AppError::Other(format!("ssh_profiles.json read: {e}")))?;
        let parsed: SshProfileFile = serde_json::from_str(&raw)
            .map_err(|e| AppError::Other(format!("ssh_profiles.json parse: {e}")))?;
        if parsed.schema_version != SSH_PROFILES_SCHEMA_VERSION {
            return Err(AppError::Other(format!(
                "ssh_profiles.json schema_version {} not supported by this build (expected {})",
                parsed.schema_version, SSH_PROFILES_SCHEMA_VERSION
            )));
        }
        Ok(parsed.profiles)
    }

    pub fn save(&self, profiles: &[SshProfile]) -> AppResult<()> {
        let file = SshProfileFile {
            schema_version: SSH_PROFILES_SCHEMA_VERSION,
            profiles: profiles.to_vec(),
        };
        let raw = serde_json::to_string_pretty(&file)
            .map_err(|e| AppError::Other(format!("ssh_profiles.json serialize: {e}")))?;
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::Other(format!("ssh_profiles.json mkdir: {e}")))?;
        }
        std::fs::write(&self.file_path, raw)
            .map_err(|e| AppError::Other(format!("ssh_profiles.json write: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh::profile::SshAuth;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn sample_profile() -> SshProfile {
        SshProfile {
            id: Uuid::new_v4(),
            name: "bastion-test".into(),
            host: "10.0.1.4".into(),
            port: 22,
            user: "ubuntu".into(),
            auth: SshAuth::Agent,
            jump: None,
        }
    }

    #[test]
    fn load_missing_file_returns_empty_vec() {
        let dir = TempDir::new().unwrap();
        let store = SshProfileStore::new(dir.path());
        let result = store.load().unwrap();
        assert!(result.is_empty());
        // Also verify no file was created by the load call
        assert!(!dir.path().join("ssh_profiles.json").exists());
    }

    #[test]
    fn save_then_load_round_trips_a_profile() {
        let dir = TempDir::new().unwrap();
        let store = SshProfileStore::new(dir.path());
        let profile = sample_profile();
        store.save(std::slice::from_ref(&profile)).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, profile.id);
        assert_eq!(loaded[0].name, profile.name);
        assert_eq!(loaded[0].host, profile.host);
        assert_eq!(loaded[0].port, profile.port);
        assert_eq!(loaded[0].user, profile.user);
        assert!(matches!(loaded[0].auth, SshAuth::Agent));
        assert!(loaded[0].jump.is_none());
    }

    #[test]
    fn load_rejects_unknown_schema_version() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("ssh_profiles.json"),
            r#"{"schema_version":999,"profiles":[]}"#,
        )
        .unwrap();
        let store = SshProfileStore::new(dir.path());
        let err = store.load().unwrap_err();
        assert!(err.to_string().contains("not supported"), "got: {err}");
    }

    #[test]
    fn save_writes_valid_json_with_schema_version() {
        let dir = TempDir::new().unwrap();
        let store = SshProfileStore::new(dir.path());
        store.save(&[sample_profile()]).unwrap();
        let raw = std::fs::read_to_string(dir.path().join("ssh_profiles.json")).unwrap();
        let parsed: SshProfileFile = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed.schema_version, SSH_PROFILES_SCHEMA_VERSION);
        assert_eq!(parsed.profiles.len(), 1);
    }
}
