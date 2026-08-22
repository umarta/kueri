//! Safety level per connection (Phase 1: type only; enforcement in Phase 3).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SafetyLevel {
    Off,
    Warn,
    #[default]
    ConfirmDestructive,
    ConfirmWrites,
    ConfirmDdl,
    ReadOnly,
}
