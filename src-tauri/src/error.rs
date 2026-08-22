use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

use crate::safety::{ConfirmReason, RejectReason};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("connection not found: {0}")]
    ConnectionNotFound(String),
    #[error("{0}")]
    Other(String),
    #[error("statement needs confirmation: {statement}")]
    NeedsConfirmation {
        token: String,
        statement: String,
        reason: ConfirmReason,
    },
    #[error("safety rejected: {statement}")]
    SafetyRejected {
        statement: String,
        reason: RejectReason,
    },
}

// Tauri command errors must be serializable to reach the frontend.
// Existing variants serialize as plain strings (matches pre-KUE-003 wire
// format so existing error-toast sites keep working via
// `(err as { message?: string })?.message ?? String(err)`).
// New safety variants serialize as tagged JSON objects with a `message`
// field so the same fallback still yields a readable string, PLUS a `kind`
// field the frontend safety handler can pattern-match.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            AppError::NeedsConfirmation {
                token,
                statement,
                reason,
            } => {
                let mut m = s.serialize_map(Some(5))?;
                m.serialize_entry("kind", "needs-confirmation")?;
                m.serialize_entry("token", token)?;
                m.serialize_entry("statement", statement)?;
                m.serialize_entry("reason", reason)?;
                m.serialize_entry("message", &self.to_string())?;
                m.end()
            }
            AppError::SafetyRejected { statement, reason } => {
                let mut m = s.serialize_map(Some(4))?;
                m.serialize_entry("kind", "safety-rejected")?;
                m.serialize_entry("statement", statement)?;
                m.serialize_entry("reason", reason)?;
                m.serialize_entry("message", &self.to_string())?;
                m.end()
            }
            // All other variants (Db, ConnectionNotFound, Other, and any future
            // variants added here) fall through to plain-string serialization.
            // If a future variant needs object serialization, add an explicit arm above.
            _ => s.serialize_str(&self.to_string()),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod error_serde_tests {
    use crate::error::AppError;
    use crate::safety::{ConfirmReason, RejectReason};

    #[test]
    fn needs_confirmation_serializes_as_tagged_object_with_message() {
        let err = AppError::NeedsConfirmation {
            token: "abc-123".into(),
            statement: "DELETE FROM t".into(),
            reason: ConfirmReason::DestructiveNoWhere,
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "needs-confirmation");
        assert_eq!(json["token"], "abc-123");
        assert_eq!(json["statement"], "DELETE FROM t");
        assert_eq!(json["reason"], "destructive-no-where");
        assert!(json["message"]
            .as_str()
            .unwrap()
            .contains("needs confirmation"));
    }

    #[test]
    fn safety_rejected_serializes_as_tagged_object_with_message() {
        let err = AppError::SafetyRejected {
            statement: "INSERT INTO t VALUES (1)".into(),
            reason: RejectReason::ReadOnlyMode,
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "safety-rejected");
        assert_eq!(json["statement"], "INSERT INTO t VALUES (1)");
        assert_eq!(json["reason"], "read-only-mode");
        assert!(json["message"]
            .as_str()
            .unwrap()
            .contains("safety rejected"));
    }

    #[test]
    fn other_variant_still_serializes_as_string() {
        let err = AppError::Other("regular error".into());
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json, serde_json::json!("regular error"));
    }

    #[test]
    fn connection_not_found_still_serializes_as_string() {
        let err = AppError::ConnectionNotFound("uuid-here".into());
        let json = serde_json::to_value(&err).unwrap();
        assert!(matches!(&json, serde_json::Value::String(s) if s.contains("uuid-here")));
    }
}
