//! Password source abstraction.

use async_trait::async_trait;
use keyring::Entry;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

const KEYRING_SERVICE: &str = "dev.kueri.app";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PasswordSource {
    Plain,
    Keychain,
    Env {
        name: String,
    },
    #[serde(rename = "onepassword")]
    OnePassword {
        item: String,
        field: String,
    },
    Vault {
        path: String,
        field: String,
    },
    #[serde(rename = "aws-sm")]
    AwsSm {
        arn: String,
        region: String,
    },
}

/// Shared shape for CLI subprocess failures. Provider modules build one of these
/// and hand it to [`map_cli_error`] to get a uniformly-shaped `AppError`.
#[allow(dead_code)]
pub(crate) enum CliErrorKind {
    NonZeroExit { stderr: String },
    Timeout,
    NotFound { bin: &'static str },
}

#[allow(dead_code)]
pub(crate) fn map_cli_error(provider: &str, hint: &str, kind: CliErrorKind) -> AppError {
    match kind {
        CliErrorKind::Timeout => {
            AppError::Other(format!("{provider}: resolver timed out after 30s"))
        }
        CliErrorKind::NotFound { bin } => {
            AppError::Other(format!("{provider}: CLI not found on PATH — install {bin}"))
        }
        CliErrorKind::NonZeroExit { stderr } => {
            let trimmed: String = stderr.chars().take(500).collect();
            let trimmed = trimmed.trim();
            AppError::Other(format!("{provider}: {hint} — {trimmed}"))
        }
    }
}

/// A resolver for one external secret provider. Impls shell out to a vendor CLI.
#[async_trait]
pub trait SecretResolver: Send + Sync {
    async fn resolve(&self) -> AppResult<SecretString>;
}

/// Resolve a password source to the plaintext secret, right before pool creation.
/// The returned `SecretString` MUST NOT be logged; drop it as soon as the pool is built.
pub async fn resolve(source: &PasswordSource, conn_id: Uuid) -> AppResult<SecretString> {
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
        | PasswordSource::AwsSm { .. } => Err(AppError::Other(
            "external secret providers ship in Phase 5".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resolve_plain_returns_empty() {
        let out = resolve(&PasswordSource::Plain, Uuid::nil()).await.unwrap();
        use secrecy::ExposeSecret;
        assert_eq!(out.expose_secret(), "");
    }

    #[tokio::test]
    async fn resolve_env_reads_env_var() {
        std::env::set_var("KUERI_TEST_PW_VAR", "hunter2");
        let src = PasswordSource::Env {
            name: "KUERI_TEST_PW_VAR".into(),
        };
        let out = resolve(&src, Uuid::nil()).await.unwrap();
        use secrecy::ExposeSecret;
        assert_eq!(out.expose_secret(), "hunter2");
        std::env::remove_var("KUERI_TEST_PW_VAR");
    }

    #[tokio::test]
    async fn resolve_env_missing_var_errors() {
        let src = PasswordSource::Env {
            name: "KUERI_TEST_ABSENT_VAR_XYZ".into(),
        };
        let err = resolve(&src, Uuid::nil()).await.unwrap_err();
        assert!(format!("{err:?}").contains("KUERI_TEST_ABSENT_VAR_XYZ"));
    }

    #[test]
    fn map_cli_error_non_zero_exit_includes_provider_hint_and_stderr() {
        let err = map_cli_error(
            "1Password",
            "op CLI failed (is it installed and signed in?)",
            CliErrorKind::NonZeroExit {
                stderr: "session expired".into(),
            },
        );
        let msg = format!("{err:?}");
        assert!(msg.contains("1Password"));
        assert!(msg.contains("op CLI failed"));
        assert!(msg.contains("session expired"));
    }

    #[test]
    fn map_cli_error_timeout_has_fixed_shape() {
        let err = map_cli_error("Vault", "unused", CliErrorKind::Timeout);
        let msg = format!("{err:?}");
        assert!(msg.contains("Vault"));
        assert!(msg.contains("timed out after 30s"));
    }

    #[test]
    fn map_cli_error_not_found_names_binary() {
        let err = map_cli_error(
            "AWS Secrets Manager",
            "unused",
            CliErrorKind::NotFound { bin: "aws" },
        );
        let msg = format!("{err:?}");
        assert!(msg.contains("AWS Secrets Manager"));
        assert!(msg.contains("aws"));
        assert!(msg.contains("PATH"));
    }

    #[test]
    fn map_cli_error_trims_stderr_to_500_chars() {
        let long = "x".repeat(1500);
        let err = map_cli_error(
            "1Password",
            "hint",
            CliErrorKind::NonZeroExit { stderr: long },
        );
        let msg = format!("{err:?}");
        // stderr contribution capped
        let x_count = msg.matches('x').count();
        assert!(x_count <= 500, "expected <=500 x chars, got {x_count}");
    }
}
