//! One-shot v1 → v2 connection schema migration.

use std::path::{Path, PathBuf};
use keyring::Entry;
use serde_json::Value;
use uuid::Uuid;

use crate::db::connect::{ConnectionConfigV2, SCHEMA_VERSION};
use crate::db::DbKind;
use crate::error::{AppError, AppResult};
use crate::safety::SafetyLevel;
use crate::secrets::PasswordSource;
use crate::ssh::profile::{SshAuth, SshProfile, SshRef};
use crate::tls::{TlsConfig, TlsMode};

/// Fixed namespace for deterministic v5 UUID derivation from v1 string ids.
/// MUST NOT change once shipped.
pub const MIGRATION_NAMESPACE: Uuid = Uuid::from_u128(0xa06f4d31_4d6c_4e21_9ad4_2f8d1c3e4c11);

const KEYRING_SERVICE: &str = "dev.kueri.app";

pub fn backup_v1_file(src: &Path) -> AppResult<PathBuf> {
    let parent = src.parent().unwrap_or_else(|| Path::new("."));
    let stem = src.file_stem().unwrap_or_default().to_string_lossy();
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("json");

    let primary = parent.join(format!("{stem}.v1.bak.{ext}"));
    if !primary.exists() {
        std::fs::copy(src, &primary).map_err(|e| AppError::Other(format!("backup: {e}")))?;
        return Ok(primary);
    }
    for n in 1..1000 {
        let candidate = parent.join(format!("{stem}.v1.bak-{n}.{ext}"));
        if !candidate.exists() {
            std::fs::copy(src, &candidate).map_err(|e| AppError::Other(format!("backup: {e}")))?;
            return Ok(candidate);
        }
    }
    Err(AppError::Other("too many v1 backups".into()))
}

/// Re-key a keychain entry from the v1 string id to the v2 uuid-string id.
/// Best-effort: failure returns Ok(()) after logging. Users can re-enter passwords.
pub fn rekey_keychain(old_id: &str, new_id: Uuid) -> AppResult<()> {
    let new_id_str = new_id.to_string();
    if old_id == new_id_str {
        return Ok(()); // already the same
    }
    let old_entry = match Entry::new(KEYRING_SERVICE, old_id) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("keychain: could not open old entry '{old_id}': {e}");
            return Ok(());
        }
    };
    let password = match old_entry.get_password() {
        Ok(p) => p,
        Err(keyring::Error::NoEntry) => return Ok(()),
        Err(e) => {
            eprintln!("keychain: could not read old entry '{old_id}': {e}");
            return Ok(());
        }
    };
    let new_entry = Entry::new(KEYRING_SERVICE, &new_id_str)
        .map_err(|e| AppError::Other(format!("keychain new entry: {e}")))?;
    new_entry
        .set_password(&password)
        .map_err(|e| AppError::Other(format!("keychain set: {e}")))?;
    let _ = old_entry.delete_credential();
    Ok(())
}

pub fn migrate_record(v1: &Value) -> AppResult<ConnectionConfigV2> {
    let obj = v1.as_object().ok_or_else(|| AppError::Other("record not an object".into()))?;

    let old_id = obj
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Other("record missing id".into()))?;

    let id = Uuid::new_v5(&MIGRATION_NAMESPACE, old_id.as_bytes());

    let kind_str = obj
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Other("record missing kind".into()))?;
    let kind: DbKind = serde_json::from_value(Value::String(kind_str.to_string()))
        .map_err(|e| AppError::Other(format!("kind '{kind_str}': {e}")))?;

    let tls = migrate_tls(obj);
    let ssh = migrate_ssh(obj);

    Ok(ConnectionConfigV2 {
        id,
        schema_version: SCHEMA_VERSION,
        name: obj.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
        kind,
        host: obj.get("host").and_then(Value::as_str).unwrap_or_default().to_string(),
        port: obj.get("port").and_then(Value::as_u64).unwrap_or(0) as u16,
        database: obj.get("database").and_then(Value::as_str).unwrap_or_default().to_string(),
        user: obj.get("user").and_then(Value::as_str).unwrap_or_default().to_string(),
        password: PasswordSource::Keychain,
        tls,
        ssh,
        safety: SafetyLevel::default(),
        color: obj.get("color").and_then(Value::as_str).map(str::to_string),
        tags: obj.get("tag").and_then(Value::as_str).map(|t| vec![t.to_string()]).unwrap_or_default(),
        file_path: obj.get("file_path").and_then(Value::as_str).map(str::to_string),
    })
}

fn migrate_tls(obj: &serde_json::Map<String, Value>) -> Option<TlsConfig> {
    let ssl_enabled = obj.get("ssl").and_then(Value::as_bool).unwrap_or(false);
    let mode_str = obj.get("ssl_mode").and_then(Value::as_str);
    if !ssl_enabled && mode_str.is_none() && obj.get("ssl_ca").is_none() && obj.get("ssl_cert").is_none() && obj.get("ssl_key").is_none() {
        return None;
    }
    let mode = mode_str.map(parse_tls_mode).unwrap_or(TlsMode::Require);
    Some(TlsConfig {
        mode,
        ca_path: obj.get("ssl_ca").and_then(Value::as_str).map(PathBuf::from),
        cert_path: obj.get("ssl_cert").and_then(Value::as_str).map(PathBuf::from),
        key_path: obj.get("ssl_key").and_then(Value::as_str).map(PathBuf::from),
    })
}

fn parse_tls_mode(s: &str) -> TlsMode {
    match s.to_ascii_lowercase().as_str() {
        "disable" | "disabled" => TlsMode::Disable,
        "allow" => TlsMode::Allow,
        "prefer" | "preferred" => TlsMode::Prefer,
        "require" | "required" => TlsMode::Require,
        "verify-ca" | "verify_ca" => TlsMode::VerifyCa,
        "verify-full" | "verify_full" | "verify-identity" | "verify_identity" => TlsMode::VerifyFull,
        _ => TlsMode::Require,
    }
}

fn migrate_ssh(obj: &serde_json::Map<String, Value>) -> Option<SshRef> {
    if !obj.get("ssh_enabled").and_then(Value::as_bool).unwrap_or(false) {
        return None;
    }
    let key_path = obj.get("ssh_key").and_then(Value::as_str).map(PathBuf::from);
    let auth = match key_path {
        Some(path) => SshAuth::KeyFile { path, passphrase: None },
        None => SshAuth::Agent,
    };
    Some(SshRef::Inline(SshProfile {
        id: Uuid::new_v4(),
        name: "migrated".to_string(),
        host: obj.get("ssh_host").and_then(Value::as_str).unwrap_or_default().to_string(),
        port: obj.get("ssh_port").and_then(Value::as_u64).unwrap_or(22) as u16,
        user: obj.get("ssh_user").and_then(Value::as_str).unwrap_or_default().to_string(),
        auth,
        jump: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use crate::tls::TlsMode;
    use crate::secrets::PasswordSource;
    use crate::ssh::profile::{SshAuth, SshRef};
    use crate::safety::SafetyLevel;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn backup_creates_bak_file() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("connections.json");
        std::fs::File::create(&src).unwrap().write_all(b"[]").unwrap();

        let backup = backup_v1_file(&src).unwrap();
        assert!(backup.exists());
        assert!(backup.file_name().unwrap().to_string_lossy().starts_with("connections.v1.bak"));
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), "[]");
    }

    #[test]
    fn backup_names_suffix_when_bak_exists() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("connections.json");
        std::fs::File::create(&src).unwrap().write_all(b"[]").unwrap();
        let first = backup_v1_file(&src).unwrap();
        let second = backup_v1_file(&src).unwrap();
        assert_ne!(first, second);
        assert!(second.exists() && first.exists());
    }

    fn v1_minimal() -> serde_json::Value {
        json!({
            "id": "abc-123",
            "name": "local",
            "kind": "postgres",
            "host": "localhost",
            "port": 5432,
            "database": "postgres",
            "user": "postgres",
            "password": "",
            "ssl": false
        })
    }

    #[test]
    fn id_is_deterministic() {
        let m1 = migrate_record(&v1_minimal()).unwrap();
        let m2 = migrate_record(&v1_minimal()).unwrap();
        assert_eq!(m1.id, m2.id);
    }

    #[test]
    fn password_defaults_to_keychain() {
        let migrated = migrate_record(&v1_minimal()).unwrap();
        assert_eq!(migrated.password, PasswordSource::Keychain);
    }

    #[test]
    fn safety_defaults_to_confirm_destructive() {
        let migrated = migrate_record(&v1_minimal()).unwrap();
        assert_eq!(migrated.safety, SafetyLevel::ConfirmDestructive);
    }

    #[test]
    fn tls_omitted_when_ssl_false() {
        let migrated = migrate_record(&v1_minimal()).unwrap();
        assert!(migrated.tls.is_none());
    }

    #[test]
    fn tls_present_when_ssl_true() {
        let mut v1 = v1_minimal();
        v1["ssl"] = json!(true);
        v1["ssl_mode"] = json!("require");
        let migrated = migrate_record(&v1).unwrap();
        let tls = migrated.tls.expect("tls");
        assert_eq!(tls.mode, TlsMode::Require);
    }

    #[test]
    fn tls_paths_carried_over() {
        let mut v1 = v1_minimal();
        v1["ssl"] = json!(true);
        v1["ssl_mode"] = json!("verify-full");
        v1["ssl_ca"] = json!("/tmp/ca.pem");
        v1["ssl_cert"] = json!("/tmp/client.crt");
        v1["ssl_key"] = json!("/tmp/client.key");
        let migrated = migrate_record(&v1).unwrap();
        let tls = migrated.tls.unwrap();
        assert_eq!(tls.mode, TlsMode::VerifyFull);
        assert_eq!(tls.ca_path.as_deref(), Some(std::path::Path::new("/tmp/ca.pem")));
        assert_eq!(tls.cert_path.as_deref(), Some(std::path::Path::new("/tmp/client.crt")));
        assert_eq!(tls.key_path.as_deref(), Some(std::path::Path::new("/tmp/client.key")));
    }

    #[test]
    fn ssh_disabled_becomes_none() {
        let migrated = migrate_record(&v1_minimal()).unwrap();
        assert!(migrated.ssh.is_none());
    }

    #[test]
    fn ssh_enabled_becomes_inline_profile() {
        let mut v1 = v1_minimal();
        v1["ssh_enabled"] = json!(true);
        v1["ssh_host"] = json!("jump.example.com");
        v1["ssh_port"] = json!(22);
        v1["ssh_user"] = json!("ec2-user");
        v1["ssh_key"] = json!("/tmp/id_rsa");
        let migrated = migrate_record(&v1).unwrap();
        let SshRef::Inline(profile) = migrated.ssh.expect("ssh") else {
            panic!("expected inline profile");
        };
        assert_eq!(profile.host, "jump.example.com");
        assert_eq!(profile.user, "ec2-user");
        assert!(matches!(profile.auth, SshAuth::KeyFile { .. }));
    }

    #[test]
    fn ssh_enabled_without_key_uses_agent() {
        let mut v1 = v1_minimal();
        v1["ssh_enabled"] = json!(true);
        v1["ssh_host"] = json!("jump.example.com");
        v1["ssh_port"] = json!(22);
        v1["ssh_user"] = json!("ec2-user");
        let migrated = migrate_record(&v1).unwrap();
        let SshRef::Inline(profile) = migrated.ssh.unwrap() else { panic!() };
        assert!(matches!(profile.auth, SshAuth::Agent));
    }

    #[test]
    fn schema_version_is_two() {
        let migrated = migrate_record(&v1_minimal()).unwrap();
        assert_eq!(migrated.schema_version, 2);
    }

    #[test]
    fn missing_id_errors() {
        let mut v1 = v1_minimal();
        v1.as_object_mut().unwrap().remove("id");
        let err = migrate_record(&v1).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("id"));
    }

    #[test]
    fn color_and_tags_preserved() {
        let mut v1 = v1_minimal();
        v1["color"] = json!("prod");
        v1["tag"] = json!("production");
        let migrated = migrate_record(&v1).unwrap();
        assert_eq!(migrated.color.as_deref(), Some("prod"));
        assert!(migrated.tags.iter().any(|t| t == "production"));
    }
}
