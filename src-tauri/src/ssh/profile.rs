//! SSH profile definitions (Phase 1: types + serialization only).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::secrets::PasswordSource;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SshAuth {
    Password {
        source: PasswordSource,
    },
    KeyFile {
        path: PathBuf,
        #[serde(default)]
        passphrase: Option<PasswordSource>,
    },
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SshProfile {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: SshAuth,
    #[serde(default)]
    pub jump: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum SshRef {
    Profile(Uuid),
    Inline(SshProfile),
}
