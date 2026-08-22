use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::safety::SafetyLevel;
use crate::secrets::PasswordSource;
use crate::ssh::profile::SshRef;
use crate::tls::TlsConfig;
use super::DbKind;

pub const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ConnectionConfigV2 {
    pub id: Uuid,
    pub schema_version: u32,
    pub name: String,
    pub kind: DbKind,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: PasswordSource,
    #[serde(default)]
    pub tls: Option<TlsConfig>,
    #[serde(default)]
    pub ssh: Option<SshRef>,
    #[serde(default)]
    pub safety: SafetyLevel,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub file_path: Option<String>,
}

/// Temporary alias so downstream code still parses while we work through Tasks 4–8.
/// Removed at the end of Task 8.
pub type ConnectionConfig = ConnectionConfigV2;

// TODO(Task 7): Rewrite URL builders to use the new field shapes (TlsConfig, PasswordSource).
// Stub bodies are placeholders so downstream callers (postgres.rs / mysql.rs / sqlite.rs)
// continue to compile. Task 7 replaces these with real implementations.
impl ConnectionConfigV2 {
    pub fn pg_url(&self) -> String {
        unimplemented!("TODO(Task 7): build postgres URL from TlsConfig + PasswordSource")
    }

    pub fn mysql_url(&self) -> String {
        unimplemented!("TODO(Task 7): build mysql URL from TlsConfig + PasswordSource")
    }

    pub fn sqlite_url(&self) -> String {
        let path = self
            .file_path
            .clone()
            .unwrap_or_else(|| self.database.clone());
        format!("sqlite://{}", path)
    }
}

/// Minimal percent-encoding for URL credentials (encodes each non-safe byte).
/// Trimmed string slice if the option holds non-blank text, else None.
fn nonempty(o: &Option<String>) -> Option<&str> {
    o.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

fn enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
