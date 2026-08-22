use serde::{Deserialize, Serialize};
use secrecy::ExposeSecret;
use uuid::Uuid;

use crate::safety::SafetyLevel;
use crate::secrets::PasswordSource;
use crate::ssh::profile::SshRef;
use crate::tls::{TlsConfig, TlsMode};
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

impl ConnectionConfigV2 {
    pub fn pg_url(&self, secret: &secrecy::SecretString) -> String {
        let sslmode = self.tls
            .as_ref()
            .map(|t| pg_mode_str(&t.mode))
            .unwrap_or("prefer");
        let mut url = format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            enc(&self.user),
            enc(secret.expose_secret()),
            self.host,
            self.port,
            self.database,
            sslmode
        );
        if let Some(tls) = &self.tls {
            if let Some(ca) = &tls.ca_path {
                url.push_str(&format!("&sslrootcert={}", enc(&ca.to_string_lossy())));
            }
            if let Some(cert) = &tls.cert_path {
                url.push_str(&format!("&sslcert={}", enc(&cert.to_string_lossy())));
            }
            if let Some(key) = &tls.key_path {
                url.push_str(&format!("&sslkey={}", enc(&key.to_string_lossy())));
            }
        }
        url
    }

    pub fn mysql_url(&self, secret: &secrecy::SecretString) -> String {
        let mode = self.tls
            .as_ref()
            .map(|t| mysql_mode_str(&t.mode))
            .unwrap_or("PREFERRED");
        let mut url = format!(
            "mysql://{}:{}@{}:{}/{}?ssl-mode={}",
            enc(&self.user),
            enc(secret.expose_secret()),
            self.host,
            self.port,
            self.database,
            mode
        );
        if let Some(tls) = &self.tls {
            if let Some(ca) = &tls.ca_path {
                url.push_str(&format!("&ssl-ca={}", enc(&ca.to_string_lossy())));
            }
        }
        url
    }

    pub fn sqlite_url(&self) -> String {
        let path = self.file_path.clone().unwrap_or_else(|| self.database.clone());
        format!("sqlite://{}", path)
    }
}

fn pg_mode_str(m: &TlsMode) -> &'static str {
    match m {
        TlsMode::Disable => "disable",
        TlsMode::Allow => "allow",
        TlsMode::Prefer => "prefer",
        TlsMode::Require => "require",
        TlsMode::VerifyCa => "verify-ca",
        TlsMode::VerifyFull => "verify-full",
    }
}

fn mysql_mode_str(m: &TlsMode) -> &'static str {
    match m {
        TlsMode::Disable => "DISABLED",
        TlsMode::Allow | TlsMode::Prefer => "PREFERRED",
        TlsMode::Require => "REQUIRED",
        TlsMode::VerifyCa => "VERIFY_CA",
        TlsMode::VerifyFull => "VERIFY_IDENTITY",
    }
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
