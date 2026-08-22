//! AWS Secrets Manager resolver via the `aws` CLI.

use async_trait::async_trait;
use secrecy::SecretString;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use super::{map_cli_error, CliErrorKind, SecretResolver};
use crate::error::AppResult;

const PROVIDER: &str = "AWS Secrets Manager";
const HINT: &str = "aws CLI failed (check credentials and ARN)";
const BIN: &str = "aws";

pub struct AwsSmResolver<'a> {
    pub arn: &'a str,
    pub region: &'a str,
}

impl AwsSmResolver<'_> {
    pub(crate) fn build_command(&self) -> Command {
        let mut cmd = Command::new(BIN);
        cmd.arg("secretsmanager")
            .arg("get-secret-value")
            .arg("--secret-id")
            .arg(self.arn)
            .arg("--region")
            .arg(self.region)
            .arg("--query")
            .arg("SecretString")
            .arg("--output")
            .arg("text");
        cmd.kill_on_drop(true);
        cmd
    }
}

#[async_trait]
impl SecretResolver for AwsSmResolver<'_> {
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
    fn build_command_uses_aws_secretsmanager_get_secret_value() {
        let r = AwsSmResolver {
            arn: "arn:aws:secretsmanager:us-east-1:123456789012:secret:pg-prod-Ab12Cd",
            region: "us-east-1",
        };
        let cmd = r.build_command();
        let inner = cmd.as_std();
        assert_eq!(inner.get_program(), "aws");
        let args: Vec<&str> = inner.get_args().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(
            args,
            vec![
                "secretsmanager",
                "get-secret-value",
                "--secret-id",
                "arn:aws:secretsmanager:us-east-1:123456789012:secret:pg-prod-Ab12Cd",
                "--region",
                "us-east-1",
                "--query",
                "SecretString",
                "--output",
                "text",
            ]
        );
    }

    #[test]
    fn build_command_puts_region_in_flag_not_env() {
        // --region <region> means we do NOT need AWS_REGION env set
        let r = AwsSmResolver {
            arn: "arn:x",
            region: "eu-west-2",
        };
        let cmd = r.build_command();
        let args: Vec<&str> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_str().unwrap())
            .collect();
        let region_idx = args.iter().position(|a| *a == "--region").unwrap();
        assert_eq!(args[region_idx + 1], "eu-west-2");
    }
}
