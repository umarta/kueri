//! HashiCorp Vault resolver via the `vault` CLI.

use async_trait::async_trait;
use secrecy::SecretString;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use super::{map_cli_error, CliErrorKind, SecretResolver};
use crate::error::AppResult;

const PROVIDER: &str = "Vault";
const HINT: &str = "vault CLI failed (is VAULT_ADDR/VAULT_TOKEN set?)";
const BIN: &str = "vault";

pub struct VaultResolver<'a> {
    pub path: &'a str,
    pub field: &'a str,
}

impl VaultResolver<'_> {
    pub(crate) fn build_command(&self) -> Command {
        let mut cmd = Command::new(BIN);
        cmd.arg("kv")
            .arg("get")
            .arg(format!("-field={}", self.field))
            .arg(self.path);
        cmd.kill_on_drop(true);
        cmd
    }
}

#[async_trait]
impl SecretResolver for VaultResolver<'_> {
    async fn resolve(&self) -> AppResult<SecretString> {
        let mut cmd = self.build_command();
        let fut = cmd.output();
        let output = match timeout(Duration::from_secs(30), fut).await {
            Err(_) => return Err(map_cli_error(PROVIDER, HINT, CliErrorKind::Timeout)),
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(map_cli_error(
                    PROVIDER,
                    HINT,
                    CliErrorKind::NotFound { bin: BIN },
                ));
            }
            Ok(Err(e)) => {
                return Err(map_cli_error(
                    PROVIDER,
                    HINT,
                    CliErrorKind::NonZeroExit {
                        stderr: e.to_string(),
                    },
                ));
            }
            Ok(Ok(o)) => o,
        };
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(map_cli_error(
                PROVIDER,
                HINT,
                CliErrorKind::NonZeroExit { stderr },
            ));
        }
        let s = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        Ok(SecretString::new(s.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_command_uses_vault_kv_get_field() {
        let r = VaultResolver {
            path: "secret/data/prod/pg",
            field: "password",
        };
        let cmd = r.build_command();
        let inner = cmd.as_std();
        assert_eq!(inner.get_program(), "vault");
        let args: Vec<&str> = inner.get_args().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(
            args,
            vec!["kv", "get", "-field=password", "secret/data/prod/pg"]
        );
    }

    #[test]
    fn build_command_field_flag_uses_equal_form() {
        // vault expects "-field=X", not "-field X"
        let r = VaultResolver {
            path: "p",
            field: "custom",
        };
        let cmd = r.build_command();
        let args: Vec<&str> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_str().unwrap())
            .collect();
        assert!(args.contains(&"-field=custom"));
    }
}
