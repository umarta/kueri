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
    /// TLS mode override (Postgres: disable/allow/prefer/require/verify-ca/verify-full;
    /// MySQL: DISABLED/PREFERRED/REQUIRED/VERIFY_CA/VERIFY_IDENTITY). Empty = derive from `ssl`.
    #[serde(default)]
    pub ssl_mode: Option<String>,
    /// Optional TLS file paths (CA / client cert / client key).
    #[serde(default)]
    pub ssl_ca: Option<String>,
    #[serde(default)]
    pub ssl_cert: Option<String>,
    #[serde(default)]
    pub ssl_key: Option<String>,
    /// SQLite only: path to the .db file.
    #[serde(default)]
    pub file_path: Option<String>,

    // ── SSH tunnel (optional) ────────────────────────────────────────────────
    #[serde(default)]
    pub ssh_enabled: bool,
    #[serde(default)]
    pub ssh_host: String,
    #[serde(default)]
    pub ssh_port: u16,
    #[serde(default)]
    pub ssh_user: String,
    /// Path to a private key (key/agent auth only — no password prompts).
    #[serde(default)]
    pub ssh_key: Option<String>,
}

impl ConnectionConfig {
    pub fn pg_url(&self) -> String {
        let sslmode = nonempty(&self.ssl_mode)
            .map(str::to_string)
            .unwrap_or_else(|| {
                if self.ssl {
                    "require".into()
                } else {
                    "prefer".into()
                }
            });
        let mut url = format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            enc(&self.user),
            enc(&self.password),
            self.host,
            self.port,
            self.database,
            sslmode
        );
        if let Some(ca) = nonempty(&self.ssl_ca) {
            url.push_str(&format!("&sslrootcert={}", enc(ca)));
        }
        if let Some(cert) = nonempty(&self.ssl_cert) {
            url.push_str(&format!("&sslcert={}", enc(cert)));
        }
        if let Some(key) = nonempty(&self.ssl_key) {
            url.push_str(&format!("&sslkey={}", enc(key)));
        }
        url
    }

    pub fn mysql_url(&self) -> String {
        let mode = nonempty(&self.ssl_mode)
            .map(str::to_string)
            .unwrap_or_else(|| {
                if self.ssl {
                    "REQUIRED".into()
                } else {
                    "PREFERRED".into()
                }
            });
        let mut url = format!(
            "mysql://{}:{}@{}:{}/{}?ssl-mode={}",
            enc(&self.user),
            enc(&self.password),
            self.host,
            self.port,
            self.database,
            mode
        );
        // sqlx MySQL takes the CA via the URL; client cert/key aren't URL-configurable.
        if let Some(ca) = nonempty(&self.ssl_ca) {
            url.push_str(&format!("&ssl-ca={}", enc(ca)));
        }
        url
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
