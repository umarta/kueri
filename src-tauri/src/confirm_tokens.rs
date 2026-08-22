//! Per-process store of in-flight confirmation tokens for Safe Mode.
//! Tokens are single-use, keyed to (connection_id, sql), with a 60s TTL.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use uuid::Uuid;

use crate::error::{AppError, AppResult};

pub const TOKEN_TTL: Duration = Duration::from_secs(60);

pub struct ConfirmTokenStore {
    tokens: Mutex<HashMap<String, PendingConfirm>>,
}

struct PendingConfirm {
    connection_id: Uuid,
    sql: String,
    issued_at: Instant,
}

impl ConfirmTokenStore {
    pub fn new() -> Self {
        Self { tokens: Mutex::new(HashMap::new()) }
    }

    pub fn issue(&self, conn_id: Uuid, sql: String) -> String {
        let token = Uuid::new_v4().to_string();
        let mut lock = self.tokens.lock().unwrap();
        // Opportunistic GC of expired entries
        lock.retain(|_, p| p.issued_at.elapsed() < TOKEN_TTL);
        lock.insert(token.clone(), PendingConfirm {
            connection_id: conn_id,
            sql,
            issued_at: Instant::now(),
        });
        token
    }

    pub fn consume(&self, token: &str) -> AppResult<(Uuid, String)> {
        let mut lock = self.tokens.lock().unwrap();
        let entry = lock.remove(token).ok_or_else(||
            AppError::Other("confirmation token invalid or already used".into()))?;
        if entry.issued_at.elapsed() > TOKEN_TTL {
            return Err(AppError::Other("confirmation token expired".into()));
        }
        Ok((entry.connection_id, entry.sql))
    }
}

impl Default for ConfirmTokenStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_and_consume_round_trip() {
        let store = ConfirmTokenStore::new();
        let conn = Uuid::new_v4();
        let sql = "DELETE FROM t".to_string();
        let token = store.issue(conn, sql.clone());
        assert!(!token.is_empty(), "token should be non-empty");
        let (got_conn, got_sql) = store.consume(&token).unwrap();
        assert_eq!(got_conn, conn);
        assert_eq!(got_sql, sql);
    }

    #[test]
    fn consume_is_single_use() {
        let store = ConfirmTokenStore::new();
        let token = store.issue(Uuid::new_v4(), "SELECT 1".into());
        assert!(store.consume(&token).is_ok());
        assert!(store.consume(&token).is_err(), "second consume must error");
    }

    #[test]
    fn consume_unknown_token_errors() {
        let store = ConfirmTokenStore::new();
        let err = store.consume("nonexistent-token").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("invalid"));
    }

    #[test]
    fn issued_tokens_are_unique() {
        let store = ConfirmTokenStore::new();
        let t1 = store.issue(Uuid::new_v4(), "SELECT 1".into());
        let t2 = store.issue(Uuid::new_v4(), "SELECT 2".into());
        assert_ne!(t1, t2, "different issues must return different tokens");
    }

    // Note: expired-token test would need to fast-forward Instant, which is
    // not easily doable in std. We rely on the manual smoke test in Task 8
    // plus code review for TTL correctness.
}
