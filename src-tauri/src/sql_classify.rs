//! SQL statement classifier — Phase 2 uses only the `Ddl` variant for
//! cache-invalidation. Phase 3 (Safe Mode) will extend this same module.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DdlKind {
    CreateTable,
    DropTable,
    AlterTable,
    CreateSchema,
    DropSchema,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlEffect {
    Read,
    Write,
    Ddl(DdlKind),
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

#[allow(dead_code)]
pub fn classify(sql: &str) -> Vec<SqlEffect> {
    let stripped = strip_comments_and_strings(sql);
    split_statements(&stripped)
        .iter()
        .map(|s| classify_one(s))
        .collect()
}

pub fn classify_one(statement: &str) -> SqlEffect {
    let stripped = strip_comments_and_strings(statement);
    let mut tokens = stripped
        .split_ascii_whitespace()
        .map(|t| t.to_ascii_uppercase());

    let verb = tokens.next().unwrap_or_default();
    let obj_type = tokens.next().unwrap_or_default();

    match verb.as_str() {
        "SELECT" | "SHOW" | "EXPLAIN" | "WITH" | "PRAGMA" | "VALUES" | "DESCRIBE" | "DESC" => {
            SqlEffect::Read
        }
        "INSERT" | "UPDATE" | "DELETE" | "REPLACE" | "MERGE" => SqlEffect::Write,
        "CREATE" => match obj_type.as_str() {
            "TABLE" | "TEMPORARY" => SqlEffect::Ddl(DdlKind::CreateTable),
            "SCHEMA" | "DATABASE" => SqlEffect::Ddl(DdlKind::CreateSchema),
            _ => SqlEffect::Ddl(DdlKind::Other),
        },
        "DROP" => match obj_type.as_str() {
            "TABLE" => SqlEffect::Ddl(DdlKind::DropTable),
            "SCHEMA" | "DATABASE" => SqlEffect::Ddl(DdlKind::DropSchema),
            _ => SqlEffect::Ddl(DdlKind::Other),
        },
        "ALTER" => match obj_type.as_str() {
            "TABLE" => SqlEffect::Ddl(DdlKind::AlterTable),
            _ => SqlEffect::Ddl(DdlKind::Other),
        },
        "TRUNCATE" | "RENAME" => SqlEffect::Ddl(DdlKind::Other),
        _ => SqlEffect::Unknown,
    }
}

/// Extract the DDL target object name from a single statement.
/// Returns `(schema, name)` where `schema` is `None` if unqualified.
/// Returns `None` if the statement has no target (too short or unrecognised).
///
/// Handles `IF [NOT] EXISTS` between the object-type keyword and the name.
/// Falls back to `None` on anything it cannot parse; callers treat `None`
/// as "unknown target" and fall back to full-connection invalidation.
pub fn extract_ddl_target(stmt: &str) -> Option<(Option<String>, String)> {
    let stripped = strip_comments_and_strings(stmt);
    let tokens: Vec<&str> = stripped.split_ascii_whitespace().collect();

    // Minimum: verb + obj_type + name  (e.g. "DROP TABLE t")
    if tokens.len() < 3 {
        return None;
    }

    // tokens[0] = verb, tokens[1] = obj_type, then optional IF [NOT] EXISTS
    let mut i = 2usize;

    if tokens.get(i).map(|t| t.eq_ignore_ascii_case("IF")) == Some(true) {
        i += 1; // skip IF
        if tokens.get(i).map(|t| t.eq_ignore_ascii_case("NOT")) == Some(true) {
            i += 1; // skip NOT
        }
        if tokens.get(i).map(|t| t.eq_ignore_ascii_case("EXISTS")) == Some(true) {
            i += 1; // skip EXISTS
        }
    }

    let raw = tokens.get(i)?;
    // Strip trailing `(` or `;` and surrounding quote characters
    let name = raw
        .trim_end_matches(['(', ';'])
        .trim_matches(['`', '"', '\'']);

    if name.is_empty() {
        return None;
    }

    if let Some(dot) = name.find('.') {
        let schema = name[..dot].trim_matches(['`', '"', '\'']).to_string();
        let tbl = name[dot + 1..].trim_matches(['`', '"', '\'']).to_string();
        if schema.is_empty() || tbl.is_empty() {
            return None;
        }
        Some((Some(schema), tbl))
    } else {
        Some((None, name.to_string()))
    }
}

/// Parsed representation of a DDL statement's kind and target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DdlStatement {
    pub kind: DdlKind,
    /// Schema name, if the DDL target was schema-qualified (e.g. `public.users`).
    pub schema: Option<String>,
    /// Object name (table name, schema name, etc.).
    pub name: String,
}

/// Return one `DdlStatement` for every DDL statement in `sql`.
/// Non-DDL statements are filtered out. An empty vec means no DDL was found.
pub fn classify_ddl_statements(sql: &str) -> Vec<DdlStatement> {
    split_statements(sql)
        .into_iter()
        .filter_map(|stmt| {
            if let SqlEffect::Ddl(kind) = classify_one(&stmt) {
                let target = extract_ddl_target(&stmt);
                Some(DdlStatement {
                    kind,
                    schema: target.as_ref().and_then(|(s, _)| s.clone()),
                    name: target.map(|(_, n)| n).unwrap_or_default(),
                })
            } else {
                None
            }
        })
        .collect()
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
        assert_eq!(
            classify("CREATE TABLE t (x int)"),
            vec![SqlEffect::Ddl(DdlKind::CreateTable)]
        );
        assert_eq!(
            classify("DROP TABLE t"),
            vec![SqlEffect::Ddl(DdlKind::DropTable)]
        );
        assert_eq!(
            classify("ALTER TABLE t ADD COLUMN y int"),
            vec![SqlEffect::Ddl(DdlKind::AlterTable)]
        );
        assert_eq!(classify("TRUNCATE t"), vec![SqlEffect::Ddl(DdlKind::Other)]);
        assert_eq!(
            classify("RENAME TABLE a TO b"),
            vec![SqlEffect::Ddl(DdlKind::Other)]
        );
    }

    #[test]
    fn multi_statement_returns_per_statement_effects() {
        let effects = classify("SELECT 1; INSERT INTO t VALUES (1); CREATE TABLE u (x int)");
        assert_eq!(
            effects,
            vec![
                SqlEffect::Read,
                SqlEffect::Write,
                SqlEffect::Ddl(DdlKind::CreateTable)
            ]
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

    #[test]
    fn ddl_kind_create_table() {
        assert_eq!(
            classify_one("CREATE TABLE t (x int)"),
            SqlEffect::Ddl(DdlKind::CreateTable)
        );
        assert_eq!(
            classify_one("create table IF NOT EXISTS public.users (id int)"),
            SqlEffect::Ddl(DdlKind::CreateTable)
        );
    }

    #[test]
    fn ddl_kind_drop_table() {
        assert_eq!(
            classify_one("DROP TABLE t"),
            SqlEffect::Ddl(DdlKind::DropTable)
        );
        assert_eq!(
            classify_one("drop table if exists public.users"),
            SqlEffect::Ddl(DdlKind::DropTable)
        );
    }

    #[test]
    fn ddl_kind_alter_table() {
        assert_eq!(
            classify_one("ALTER TABLE t ADD COLUMN y int"),
            SqlEffect::Ddl(DdlKind::AlterTable)
        );
    }

    #[test]
    fn ddl_kind_create_schema() {
        assert_eq!(
            classify_one("CREATE SCHEMA myschema"),
            SqlEffect::Ddl(DdlKind::CreateSchema)
        );
        assert_eq!(
            classify_one("CREATE DATABASE mydb"),
            SqlEffect::Ddl(DdlKind::CreateSchema)
        );
    }

    #[test]
    fn ddl_kind_drop_schema() {
        assert_eq!(
            classify_one("DROP SCHEMA myschema"),
            SqlEffect::Ddl(DdlKind::DropSchema)
        );
        assert_eq!(
            classify_one("DROP DATABASE mydb"),
            SqlEffect::Ddl(DdlKind::DropSchema)
        );
    }

    #[test]
    fn ddl_kind_other_for_truncate_and_rename() {
        assert_eq!(classify_one("TRUNCATE t"), SqlEffect::Ddl(DdlKind::Other));
        assert_eq!(
            classify_one("RENAME TABLE a TO b"),
            SqlEffect::Ddl(DdlKind::Other)
        );
    }

    #[test]
    fn extract_target_simple_unqualified() {
        // No schema prefix — returns (None, table_name)
        assert_eq!(
            extract_ddl_target("CREATE TABLE users (id int)"),
            Some((None, "users".to_string()))
        );
    }

    #[test]
    fn extract_target_schema_qualified() {
        assert_eq!(
            extract_ddl_target("CREATE TABLE public.users (id int)"),
            Some((Some("public".to_string()), "users".to_string()))
        );
    }

    #[test]
    fn extract_target_if_not_exists() {
        assert_eq!(
            extract_ddl_target("CREATE TABLE IF NOT EXISTS public.users (id int)"),
            Some((Some("public".to_string()), "users".to_string()))
        );
    }

    #[test]
    fn extract_target_if_exists() {
        assert_eq!(
            extract_ddl_target("DROP TABLE IF EXISTS public.users"),
            Some((Some("public".to_string()), "users".to_string()))
        );
    }

    #[test]
    fn extract_target_alter_table() {
        assert_eq!(
            extract_ddl_target("ALTER TABLE public.users ADD COLUMN x INT"),
            Some((Some("public".to_string()), "users".to_string()))
        );
    }

    #[test]
    fn extract_target_returns_none_on_incomplete_stmt() {
        // Only verb + object_type, no name
        assert_eq!(extract_ddl_target("CREATE TABLE"), None);
        assert_eq!(extract_ddl_target("DROP"), None);
    }

    #[test]
    fn classify_ddl_statements_empty_on_no_ddl() {
        assert!(classify_ddl_statements("SELECT 1").is_empty());
        assert!(classify_ddl_statements("INSERT INTO t VALUES (1)").is_empty());
        assert!(classify_ddl_statements("").is_empty());
    }

    #[test]
    fn classify_ddl_statements_single_create_table() {
        let stmts = classify_ddl_statements("CREATE TABLE public.users (id int)");
        assert_eq!(stmts.len(), 1);
        assert_eq!(stmts[0].kind, DdlKind::CreateTable);
        assert_eq!(stmts[0].schema, Some("public".to_string()));
        assert_eq!(stmts[0].name, "users".to_string());
    }

    #[test]
    fn classify_ddl_statements_filters_non_ddl_in_multi_statement() {
        let stmts = classify_ddl_statements(
            "SELECT 1; ALTER TABLE public.users ADD COLUMN x INT; SELECT 2",
        );
        assert_eq!(stmts.len(), 1);
        assert_eq!(stmts[0].kind, DdlKind::AlterTable);
        assert_eq!(stmts[0].schema, Some("public".to_string()));
        assert_eq!(stmts[0].name, "users".to_string());
    }

    #[test]
    fn classify_ddl_statements_unqualified_table() {
        let stmts = classify_ddl_statements("CREATE TABLE users (id int)");
        assert_eq!(stmts.len(), 1);
        assert_eq!(stmts[0].schema, None);
        assert_eq!(stmts[0].name, "users".to_string());
    }

    #[test]
    fn classify_ddl_statements_multiple_ddl() {
        let stmts = classify_ddl_statements("CREATE TABLE public.a (x int); DROP TABLE public.b");
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0].kind, DdlKind::CreateTable);
        assert_eq!(stmts[0].name, "a".to_string());
        assert_eq!(stmts[1].kind, DdlKind::DropTable);
        assert_eq!(stmts[1].name, "b".to_string());
    }
}
