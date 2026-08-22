//! 1Password resolver via the `op` CLI.

use async_trait::async_trait;
use secrecy::SecretString;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use super::{map_cli_error, CliErrorKind, SecretResolver};
use crate::error::AppResult;

const PROVIDER: &str = "1Password";
const HINT: &str = "op CLI failed (is it installed and signed in?)";
const BIN: &str = "op";

pub struct OnePasswordResolver<'a> {
    pub item: &'a str,
    pub field: &'a str,
}

impl OnePasswordResolver<'_> {
    /// Build (but don't spawn) the exact CLI command this resolver would run.
    /// Split out for unit testing without invoking the real binary.
    pub(crate) fn build_command(&self) -> Command {
        let mut cmd = Command::new(BIN);
        cmd.arg("item")
            .arg("get")
            .arg(self.item)
            .arg("--fields")
            .arg(self.field)
            .arg("--reveal");
        cmd.kill_on_drop(true);
        cmd
    }
}

#[async_trait]
impl SecretResolver for OnePasswordResolver<'_> {
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
    fn build_command_uses_op_item_get_reveal() {
        let r = OnePasswordResolver {
            item: "Postgres Prod",
            field: "password",
        };
        let cmd = r.build_command();
        let inner = cmd.as_std();
        assert_eq!(inner.get_program(), "op");
        let args: Vec<&str> = inner.get_args().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(
            args,
            vec![
                "item",
                "get",
                "Postgres Prod",
                "--fields",
                "password",
                "--reveal"
            ]
        );
    }

    #[test]
    fn build_command_accepts_uuid_as_item() {
        let r = OnePasswordResolver {
            item: "abc123def456ghi789jkl012mn",
            field: "password",
        };
        let cmd = r.build_command();
        let args: Vec<&str> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_str().unwrap())
            .collect();
        assert_eq!(args[2], "abc123def456ghi789jkl012mn");
    }
}
