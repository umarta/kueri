//! Password source abstraction (Phase 1: enum + skeleton; resolvers land in Phase 5).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PasswordSource {
    /// Plaintext lives only in the current session (memory).
    Plain,
    /// Resolved from the OS keychain via the `keyring` crate.
    Keychain,
    /// Resolved from an environment variable at connect time.
    Env { name: String },
    /// Reserved for Phase 5.
    #[serde(rename = "onepassword")]
    OnePassword { item: String, field: String },
    /// Reserved for Phase 5.
    Vault { path: String, field: String },
    /// Reserved for Phase 5.
    #[serde(rename = "aws-sm")]
    AwsSm { arn: String, region: String },
}
