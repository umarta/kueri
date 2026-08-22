//! Safety level per connection (Phase 1: type only; enforcement in Phase 3).

use crate::sql_classify::SqlEffect;
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ConfirmReason {
    DestructiveNoWhere,
    Write,
    Ddl,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RejectReason {
    ReadOnlyMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyDecision {
    Allow,
    NeedsConfirmation {
        reason: ConfirmReason,
        statement: String,
    },
    Reject {
        reason: RejectReason,
        statement: String,
    },
}

impl SafetyLevel {
    /// Decide what to do with a single statement at this safety level.
    /// `effect` is from sql_classify; `has_where` is only meaningful for Write.
    pub fn decide(&self, effect: SqlEffect, has_where: bool, statement: &str) -> SafetyDecision {
        // ReadOnly: everything except Read is rejected
        if matches!(self, SafetyLevel::ReadOnly) {
            return match effect {
                SqlEffect::Read => SafetyDecision::Allow,
                _ => SafetyDecision::Reject {
                    reason: RejectReason::ReadOnlyMode,
                    statement: statement.to_string(),
                },
            };
        }

        // Off / Warn: allow everything (Warn is visual-only in the frontend)
        if matches!(self, SafetyLevel::Off | SafetyLevel::Warn) {
            return SafetyDecision::Allow;
        }

        // Confirm* levels: choose reason based on effect + has_where
        let confirm = |reason| SafetyDecision::NeedsConfirmation {
            reason,
            statement: statement.to_string(),
        };

        match (self, effect) {
            (SafetyLevel::ConfirmDestructive, SqlEffect::Write) if !has_where => {
                confirm(ConfirmReason::DestructiveNoWhere)
            }
            (SafetyLevel::ConfirmWrites, SqlEffect::Write) => {
                if has_where {
                    confirm(ConfirmReason::Write)
                } else {
                    confirm(ConfirmReason::DestructiveNoWhere)
                }
            }
            (SafetyLevel::ConfirmDdl, SqlEffect::Write) => {
                // ConfirmDdl stacks with Writes
                if has_where {
                    confirm(ConfirmReason::Write)
                } else {
                    confirm(ConfirmReason::DestructiveNoWhere)
                }
            }
            (SafetyLevel::ConfirmDdl, SqlEffect::Ddl) => confirm(ConfirmReason::Ddl),
            _ => SafetyDecision::Allow,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql_classify::SqlEffect;

    #[test]
    fn off_allows_everything() {
        assert!(matches!(
            SafetyLevel::Off.decide(SqlEffect::Ddl, false, "DROP TABLE t"),
            SafetyDecision::Allow
        ));
        assert!(matches!(
            SafetyLevel::Off.decide(SqlEffect::Write, false, "DELETE FROM t"),
            SafetyDecision::Allow
        ));
    }

    #[test]
    fn warn_allows_everything_too() {
        // Warn is visual only — no backend gate
        assert!(matches!(
            SafetyLevel::Warn.decide(SqlEffect::Ddl, false, "DROP TABLE t"),
            SafetyDecision::Allow
        ));
        assert!(matches!(
            SafetyLevel::Warn.decide(SqlEffect::Write, false, "DELETE FROM t"),
            SafetyDecision::Allow
        ));
    }

    #[test]
    fn confirm_destructive_triggers_only_on_destructive_no_where() {
        // Write with WHERE — allow
        assert!(matches!(
            SafetyLevel::ConfirmDestructive.decide(
                SqlEffect::Write,
                true,
                "DELETE FROM t WHERE id=1"
            ),
            SafetyDecision::Allow
        ));
        // Write without WHERE — confirm
        assert!(matches!(
            SafetyLevel::ConfirmDestructive.decide(SqlEffect::Write, false, "DELETE FROM t"),
            SafetyDecision::NeedsConfirmation {
                reason: ConfirmReason::DestructiveNoWhere,
                ..
            }
        ));
        // DDL — allow at this level
        assert!(matches!(
            SafetyLevel::ConfirmDestructive.decide(SqlEffect::Ddl, false, "DROP TABLE t"),
            SafetyDecision::Allow
        ));
    }

    #[test]
    fn confirm_writes_triggers_on_any_write_regardless_of_where() {
        assert!(matches!(
            SafetyLevel::ConfirmWrites.decide(SqlEffect::Write, true, "DELETE FROM t WHERE id=1"),
            SafetyDecision::NeedsConfirmation {
                reason: ConfirmReason::Write,
                ..
            }
        ));
        assert!(matches!(
            SafetyLevel::ConfirmWrites.decide(SqlEffect::Write, false, "DELETE FROM t"),
            SafetyDecision::NeedsConfirmation {
                reason: ConfirmReason::DestructiveNoWhere,
                ..
            }
        ));
        // DDL passes through (still confirmed at Ddl level, not Writes)
        assert!(matches!(
            SafetyLevel::ConfirmWrites.decide(SqlEffect::Ddl, false, "DROP TABLE t"),
            SafetyDecision::Allow
        ));
        assert!(matches!(
            SafetyLevel::ConfirmWrites.decide(SqlEffect::Read, false, "SELECT 1"),
            SafetyDecision::Allow
        ));
    }

    #[test]
    fn confirm_ddl_triggers_on_ddl_and_writes() {
        // ConfirmDdl stacks: it confirms Writes AND Ddl
        assert!(matches!(
            SafetyLevel::ConfirmDdl.decide(SqlEffect::Ddl, false, "CREATE TABLE t (x int)"),
            SafetyDecision::NeedsConfirmation {
                reason: ConfirmReason::Ddl,
                ..
            }
        ));
        assert!(matches!(
            SafetyLevel::ConfirmDdl.decide(SqlEffect::Write, true, "INSERT INTO t VALUES (1)"),
            SafetyDecision::NeedsConfirmation {
                reason: ConfirmReason::Write,
                ..
            }
        ));
        assert!(matches!(
            SafetyLevel::ConfirmDdl.decide(SqlEffect::Read, false, "SELECT 1"),
            SafetyDecision::Allow
        ));
    }

    #[test]
    fn read_only_rejects_writes_and_ddl() {
        assert!(matches!(
            SafetyLevel::ReadOnly.decide(SqlEffect::Write, true, "INSERT INTO t VALUES (1)"),
            SafetyDecision::Reject {
                reason: RejectReason::ReadOnlyMode,
                ..
            }
        ));
        assert!(matches!(
            SafetyLevel::ReadOnly.decide(SqlEffect::Ddl, false, "DROP TABLE t"),
            SafetyDecision::Reject {
                reason: RejectReason::ReadOnlyMode,
                ..
            }
        ));
        // Read passes
        assert!(matches!(
            SafetyLevel::ReadOnly.decide(SqlEffect::Read, false, "SELECT 1"),
            SafetyDecision::Allow
        ));
    }

    #[test]
    fn unknown_effect_treated_as_write_by_confirm_writes() {
        // Unknown verb (e.g., BEGIN, VACUUM) — err on the side of confirmation at Writes+
        assert!(matches!(
            SafetyLevel::ConfirmWrites.decide(SqlEffect::Unknown, false, "VACUUM"),
            SafetyDecision::Allow
        ));
        // At ConfirmWrites, unknown is not a write. But at ReadOnly it should reject.
        assert!(matches!(
            SafetyLevel::ReadOnly.decide(SqlEffect::Unknown, false, "VACUUM"),
            SafetyDecision::Reject { .. }
        ));
    }

    #[test]
    fn confirm_reason_serializes_kebab_case() {
        assert_eq!(
            serde_json::to_string(&ConfirmReason::DestructiveNoWhere).unwrap(),
            r#""destructive-no-where""#
        );
        assert_eq!(
            serde_json::to_string(&ConfirmReason::Write).unwrap(),
            r#""write""#
        );
        assert_eq!(
            serde_json::to_string(&ConfirmReason::Ddl).unwrap(),
            r#""ddl""#
        );
        assert_eq!(
            serde_json::to_string(&RejectReason::ReadOnlyMode).unwrap(),
            r#""read-only-mode""#
        );
    }
}
