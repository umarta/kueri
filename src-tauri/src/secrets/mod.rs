//! Password source abstraction (Phase 1: Plain/Keychain/Env resolvers).

use std::env;
use serde::{Deserialize, Serialize};
use secrecy::SecretString;
use uuid::Uuid;
use keyring::Entry;

use crate::error::{AppError, AppResult};

const KEYRING_SERVICE: &str = "dev.kueri.app";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PasswordSource {
    Plain,
    Keychain,
    Env { name: String },
    #[serde(rename = "onepassword")]
    OnePassword { item: String, field: String },
    Vault { path: String, field: String },
    #[serde(rename = "aws-sm")]
    AwsSm { arn: String, region: String },
}

/// Resolve a password source to the plaintext secret, right before pool creation.
/// The returned `SecretString` MUST NOT be logged; drop it as soon as the pool is built.
pub fn resolve(source: &PasswordSource, conn_id: Uuid) -> AppResult<SecretString> {
    match source {
        PasswordSource::Plain => Ok(SecretString::new(String::new().into())),
        PasswordSource::Keychain => {
            let key = conn_id.to_string();
            let entry = Entry::new(KEYRING_SERVICE, &key)
                .map_err(|e| AppError::Other(format!("keychain: {e}")))?;
            match entry.get_password() {
                Ok(p) => Ok(SecretString::new(p.into())),
                Err(keyring::Error::NoEntry) => Ok(SecretString::new(String::new().into())),
                Err(e) => Err(AppError::Other(format!("keychain read: {e}"))),
            }
        }
        PasswordSource::Env { name } => match env::var(name) {
            Ok(p) => Ok(SecretString::new(p.into())),
            Err(_) => Err(AppError::Other(format!("env var {name} not set"))),
        },
        PasswordSource::OnePassword { .. }
        | PasswordSource::Vault { .. }
        | PasswordSource::AwsSm { .. } => {
            Err(AppError::Other("external secret providers ship in Phase 5".into()))
        }
    }
}
