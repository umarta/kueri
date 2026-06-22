use serde::{Deserialize, Serialize};

use super::DbKind;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub kind: DbKind,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub database: String,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub ssl: bool,
    /// SQLite only: path to the .db file.
    #[serde(default)]
    pub file_path: Option<String>,
}

impl ConnectionConfig {
    pub fn pg_url(&self) -> String {
        let sslmode = if self.ssl { "require" } else { "prefer" };
        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            enc(&self.user),
            enc(&self.password),
            self.host,
            self.port,
            self.database,
            sslmode
        )
    }

    pub fn mysql_url(&self) -> String {
        let ssl = if self.ssl { "?ssl-mode=REQUIRED" } else { "" };
        format!(
            "mysql://{}:{}@{}:{}/{}{}",
            enc(&self.user),
            enc(&self.password),
            self.host,
            self.port,
            self.database,
            ssl
        )
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
