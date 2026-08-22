//! SQL statement classifier — Phase 2 uses only the `Ddl` variant for
//! cache-invalidation. Phase 3 (Safe Mode) will extend this same module.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlEffect {
    Read,
    Write,
    Ddl,
    Unknown,
}

/// Split a multi-statement string into individual statements. Handles
/// comments and string literals correctly (uses the same stripper as classify).
pub fn split_statements(sql: &str) -> Vec<String> {
    let stripped = strip_comments_and_strings(sql);
    // Since strip_comments_and_strings preserves character offsets (replacing
    // comments/strings with spaces of same length), we can use the same positions
    // to split the original sql.
    let mut result = Vec::new();
    let mut start = 0;
    let bytes = stripped.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b';' {
            let piece = sql[start..i].trim().to_string();
            if !piece.is_empty() {
                result.push(piece);
            }
            start = i + 1;
        }
    }
    let tail = sql[start..].trim().to_string();
    if !tail.is_empty() {
        result.push(tail);
    }
    result
}

pub fn classify(sql: &str) -> Vec<SqlEffect> {
    let stripped = strip_comments_and_strings(sql);
    split_statements(&stripped)
        .iter()
        .map(|s| classify_one(s))
        .collect()
}

fn classify_one(statement: &str) -> SqlEffect {
    // Take the first alphabetic token (case-insensitive), match against verbs.
    let verb: String = statement
        .chars()
        .skip_while(|c| !c.is_ascii_alphabetic())
        .take_while(|c| c.is_ascii_alphabetic())
        .collect::<String>()
        .to_ascii_uppercase();
    match verb.as_str() {
        "SELECT" | "SHOW" | "EXPLAIN" | "WITH" | "PRAGMA" | "VALUES" | "DESCRIBE" | "DESC" => {
            SqlEffect::Read
        }
        "INSERT" | "UPDATE" | "DELETE" | "REPLACE" | "MERGE" => SqlEffect::Write,
        "CREATE" | "DROP" | "ALTER" | "TRUNCATE" | "RENAME" => SqlEffect::Ddl,
        _ => SqlEffect::Unknown,
    }
}

/// Does the statement have a WHERE clause at the top level?
/// Uses the same stripper as `classify` so comments and string literals
/// containing "WHERE" don't produce false positives.
/// Detects any occurrence of WHERE as a standalone token in the stripped text —
/// this catches WHERE in a subquery too, which is acceptable for safety
/// (over-detects → user confirms → run; safer than under-detecting).
pub fn has_where_clause(sql: &str) -> bool {
    let stripped = strip_comments_and_strings(sql);
    let upper = stripped.to_ascii_uppercase();
    // Look for WHERE surrounded by non-alphanumeric-non-underscore characters
    // (so WHERE_LIKE identifier doesn't match).
    let bytes = upper.as_bytes();
    let needle = b"WHERE";
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            let before_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
            let after_ok =
                i + needle.len() == bytes.len() || !is_ident_byte(bytes[i + needle.len()]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Strip block comments (/* ... */), line comments (-- ...), and string literals
/// ('...' and "..."), replacing them with equivalent-length whitespace. This
/// preserves original character offsets (helpful for future diagnostics) but
/// removes any content that might confuse verb detection.
fn strip_comments_and_strings(sql: &str) -> String {
    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        // Block comment
        if b == b'/' && bytes.get(i + 1) == Some(&b'*') {
            out.push_str("  ");
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                out.push(if bytes[i] == b'\n' { '\n' } else { ' ' });
                i += 1;
            }
            if i + 1 < bytes.len() {
                out.push_str("  ");
                i += 2;
            }
            continue;
        }
        // Line comment
        if b == b'-' && bytes.get(i + 1) == Some(&b'-') {
            while i < bytes.len() && bytes[i] != b'\n' {
                out.push(' ');
                i += 1;
            }
            continue;
        }
        // Single-quoted string (SQL — doubled '' is an escape)
        if b == b'\'' {
            out.push(' ');
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\'' {
                    if bytes.get(i + 1) == Some(&b'\'') {
                        out.push_str("  ");
                        i += 2;
                        continue;
                    }
                    out.push(' ');
                    i += 1;
                    break;
                }
                out.push(if bytes[i] == b'\n' { '\n' } else { ' ' });
                i += 1;
            }
            continue;
        }
        // Double-quoted identifier (Postgres) — treat like a string for classification
        if b == b'"' {
            out.push(' ');
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                out.push(if bytes[i] == b'\n' { '\n' } else { ' ' });
                i += 1;
            }
            if i < bytes.len() {
                out.push(' ');
                i += 1;
            }
            continue;
        }
        out.push(b as char);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_empty_vec() {
        assert!(classify("").is_empty());
        assert!(classify("   \n  ").is_empty());
    }

    #[test]
    fn select_is_read() {
        assert_eq!(classify("SELECT 1"), vec![SqlEffect::Read]);
        assert_eq!(classify("select * from users"), vec![SqlEffect::Read]);
        assert_eq!(classify("  SELECT 1  "), vec![SqlEffect::Read]);
    }

    #[test]
    fn show_explain_with_pragma_values_are_read() {
        assert_eq!(classify("SHOW TABLES"), vec![SqlEffect::Read]);
        assert_eq!(classify("EXPLAIN SELECT 1"), vec![SqlEffect::Read]);
        assert_eq!(
            classify("WITH x AS (SELECT 1) SELECT * FROM x"),
            vec![SqlEffect::Read]
        );
        assert_eq!(classify("PRAGMA table_info(t)"), vec![SqlEffect::Read]);
        assert_eq!(classify("VALUES (1), (2)"), vec![SqlEffect::Read]);
    }

    #[test]
    fn insert_update_delete_are_write() {
        assert_eq!(classify("INSERT INTO t VALUES (1)"), vec![SqlEffect::Write]);
        assert_eq!(classify("UPDATE t SET x = 1"), vec![SqlEffect::Write]);
        assert_eq!(classify("DELETE FROM t"), vec![SqlEffect::Write]);
    }

    #[test]
    fn create_drop_alter_truncate_rename_are_ddl() {
        assert_eq!(classify("CREATE TABLE t (x int)"), vec![SqlEffect::Ddl]);
        assert_eq!(classify("DROP TABLE t"), vec![SqlEffect::Ddl]);
        assert_eq!(
            classify("ALTER TABLE t ADD COLUMN y int"),
            vec![SqlEffect::Ddl]
        );
        assert_eq!(classify("TRUNCATE t"), vec![SqlEffect::Ddl]);
        assert_eq!(classify("RENAME TABLE a TO b"), vec![SqlEffect::Ddl]);
    }

    #[test]
    fn multi_statement_returns_per_statement_effects() {
        let effects = classify("SELECT 1; INSERT INTO t VALUES (1); CREATE TABLE u (x int)");
        assert_eq!(
            effects,
            vec![SqlEffect::Read, SqlEffect::Write, SqlEffect::Ddl]
        );
    }

    #[test]
    fn comments_do_not_change_classification() {
        assert_eq!(
            classify("-- CREATE TABLE evil\nSELECT 1"),
            vec![SqlEffect::Read]
        );
        assert_eq!(
            classify("/* CREATE TABLE evil */ SELECT 1"),
            vec![SqlEffect::Read]
        );
    }

    #[test]
    fn string_literals_do_not_fool_the_classifier() {
        assert_eq!(classify("SELECT '/* CREATE */'"), vec![SqlEffect::Read]);
        assert_eq!(classify("SELECT 'DROP TABLE users'"), vec![SqlEffect::Read]);
        assert_eq!(classify("SELECT \"CREATE\""), vec![SqlEffect::Read]);
    }

    #[test]
    fn unknown_verb_returns_unknown() {
        assert_eq!(classify("BEGIN"), vec![SqlEffect::Unknown]);
        assert_eq!(classify("VACUUM ANALYZE"), vec![SqlEffect::Unknown]);
    }

    #[test]
    fn trailing_semicolon_does_not_add_empty_statement() {
        assert_eq!(classify("SELECT 1;"), vec![SqlEffect::Read]);
        assert_eq!(classify("SELECT 1; "), vec![SqlEffect::Read]);
    }

    #[test]
    fn has_where_true_for_delete_with_where() {
        assert!(has_where_clause("DELETE FROM t WHERE id = 1"));
    }

    #[test]
    fn has_where_false_for_delete_without_where() {
        assert!(!has_where_clause("DELETE FROM t"));
        assert!(!has_where_clause("delete from t;"));
    }

    #[test]
    fn has_where_ignores_where_in_line_comment() {
        assert!(!has_where_clause("DELETE FROM t -- WHERE id = 1"));
    }

    #[test]
    fn has_where_ignores_where_in_block_comment() {
        assert!(!has_where_clause("DELETE FROM t /* WHERE id = 1 */"));
    }

    #[test]
    fn has_where_ignores_where_in_string_literal() {
        assert!(!has_where_clause("DELETE FROM t WHERE_LIKE = 'WHERE'"));
        // Note the pre-WHERE_LIKE — a legit identifier that starts with WHERE. Should not match.
    }

    #[test]
    fn has_where_detects_where_after_subquery() {
        // The outer WHERE is what matters
        assert!(has_where_clause(
            "UPDATE t SET x = (SELECT max(y) FROM u WHERE y > 0) WHERE t.id = 1"
        ));
    }

    #[test]
    fn split_statements_handles_multi_statement() {
        let parts = split_statements("SELECT 1; DELETE FROM t; INSERT INTO t VALUES (1)");
        assert_eq!(parts.len(), 3);
        assert!(parts[0].trim_start().starts_with("SELECT"));
        assert!(parts[1].trim_start().starts_with("DELETE"));
        assert!(parts[2].trim_start().starts_with("INSERT"));
    }
}
