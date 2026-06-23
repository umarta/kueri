//! Dialect-aware DDL SQL generation. Lives in the backend so the UI never has to
//! branch on database type — components call `Driver` methods, the driver reports
//! its `Dialect`, and these builders produce the right SQL.

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Postgres,
    MySql,
    Sqlite,
    SqlServer,
}

/// A column definition coming from the table designer (matches the frontend shape).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnDef {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub nullable: bool,
    #[serde(default)]
    pub primary_key: bool,
    /// Optional default expression (empty = none). Used verbatim, e.g. `now()`, `0`, `'x'`.
    #[serde(default)]
    pub default: String,
}

fn dq(v: &str) -> String {
    format!("\"{}\"", v.replace('"', "\"\""))
}
fn bq(v: &str) -> String {
    format!("`{}`", v.replace('`', "``"))
}
pub fn ident(d: Dialect, v: &str) -> String {
    match d {
        Dialect::MySql => bq(v),
        _ => dq(v),
    }
}
/// Schema-qualified table name (SQLite has no schema namespace).
pub fn qualify(d: Dialect, schema: &str, table: &str) -> String {
    match d {
        Dialect::MySql => format!("{}.{}", bq(schema), bq(table)),
        Dialect::Sqlite => dq(table),
        _ => format!("{}.{}", dq(schema), dq(table)),
    }
}

pub fn create_table(d: Dialect, schema: &str, table: &str, cols: &[ColumnDef]) -> String {
    let mut defs: Vec<String> = cols
        .iter()
        .filter(|c| !c.name.trim().is_empty() && !c.data_type.trim().is_empty())
        .map(|c| {
            let def = c.default.trim();
            format!(
                "{} {}{}{}",
                ident(d, c.name.trim()),
                c.data_type.trim(),
                if def.is_empty() {
                    String::new()
                } else {
                    format!(" DEFAULT {def}")
                },
                if c.nullable { "" } else { " NOT NULL" }
            )
        })
        .collect();
    let pks: Vec<String> = cols
        .iter()
        .filter(|c| c.primary_key && !c.name.trim().is_empty())
        .map(|c| ident(d, c.name.trim()))
        .collect();
    if !pks.is_empty() {
        defs.push(format!("PRIMARY KEY ({})", pks.join(", ")));
    }
    format!(
        "CREATE TABLE {} (\n  {}\n);",
        qualify(d, schema, table),
        defs.join(",\n  ")
    )
}

pub fn drop_table(d: Dialect, schema: &str, table: &str) -> String {
    format!("DROP TABLE {};", qualify(d, schema, table))
}

pub fn rename_table(d: Dialect, schema: &str, old: &str, new: &str) -> String {
    match d {
        Dialect::MySql => format!(
            "RENAME TABLE {} TO {};",
            qualify(d, schema, old),
            qualify(d, schema, new)
        ),
        Dialect::SqlServer => format!(
            "EXEC sp_rename '{}.{}', '{}';",
            schema,
            old.replace('\'', "''"),
            new.replace('\'', "''")
        ),
        _ => format!(
            "ALTER TABLE {} RENAME TO {};",
            qualify(d, schema, old),
            ident(d, new)
        ),
    }
}

pub fn truncate_table(d: Dialect, schema: &str, table: &str) -> String {
    match d {
        Dialect::Sqlite => format!("DELETE FROM {};", qualify(d, schema, table)),
        _ => format!("TRUNCATE TABLE {};", qualify(d, schema, table)),
    }
}

pub fn duplicate_table(d: Dialect, schema: &str, src: &str, dst: &str) -> String {
    let from = qualify(d, schema, src);
    let to = qualify(d, schema, dst);
    match d {
        Dialect::SqlServer => format!("SELECT * INTO {to} FROM {from};"),
        Dialect::Postgres => format!("CREATE TABLE {to} AS TABLE {from};"),
        _ => format!("CREATE TABLE {to} AS SELECT * FROM {from};"),
    }
}

pub fn add_column(d: Dialect, schema: &str, table: &str, col: &ColumnDef) -> String {
    let def = col.default.trim();
    format!(
        "ALTER TABLE {} ADD COLUMN {} {}{}{};",
        qualify(d, schema, table),
        ident(d, col.name.trim()),
        col.data_type.trim(),
        if def.is_empty() {
            String::new()
        } else {
            format!(" DEFAULT {def}")
        },
        if col.nullable { "" } else { " NOT NULL" }
    )
}

pub fn drop_column(d: Dialect, schema: &str, table: &str, column: &str) -> String {
    format!(
        "ALTER TABLE {} DROP COLUMN {};",
        qualify(d, schema, table),
        ident(d, column)
    )
}

pub fn rename_column(d: Dialect, schema: &str, table: &str, old: &str, new: &str) -> String {
    match d {
        Dialect::SqlServer => format!(
            "EXEC sp_rename '{}.{}.{}', '{}', 'COLUMN';",
            schema,
            table,
            old.replace('\'', "''"),
            new.replace('\'', "''")
        ),
        _ => format!(
            "ALTER TABLE {} RENAME COLUMN {} TO {};",
            qualify(d, schema, table),
            ident(d, old),
            ident(d, new)
        ),
    }
}

pub fn change_column_type(
    d: Dialect,
    schema: &str,
    table: &str,
    column: &str,
    new_type: &str,
    not_null: bool,
) -> String {
    let q = qualify(d, schema, table);
    let c = ident(d, column);
    match d {
        Dialect::MySql => format!(
            "ALTER TABLE {q} MODIFY COLUMN {c} {new_type}{};",
            if not_null { " NOT NULL" } else { " NULL" }
        ),
        Dialect::SqlServer => format!(
            "ALTER TABLE {q} ALTER COLUMN {c} {new_type}{};",
            if not_null { " NOT NULL" } else { " NULL" }
        ),
        _ => format!("ALTER TABLE {q} ALTER COLUMN {c} TYPE {new_type} USING {c}::{new_type};"),
    }
}

pub fn set_column_nullable(
    d: Dialect,
    schema: &str,
    table: &str,
    column: &str,
    current_type: &str,
    not_null: bool,
) -> String {
    let q = qualify(d, schema, table);
    let c = ident(d, column);
    match d {
        Dialect::MySql => format!(
            "ALTER TABLE {q} MODIFY COLUMN {c} {current_type}{};",
            if not_null { " NOT NULL" } else { " NULL" }
        ),
        Dialect::SqlServer => format!(
            "ALTER TABLE {q} ALTER COLUMN {c} {current_type}{};",
            if not_null { " NOT NULL" } else { " NULL" }
        ),
        _ => {
            if not_null {
                format!("ALTER TABLE {q} ALTER COLUMN {c} SET NOT NULL;")
            } else {
                format!("ALTER TABLE {q} ALTER COLUMN {c} DROP NOT NULL;")
            }
        }
    }
}

pub fn create_index(
    d: Dialect,
    schema: &str,
    table: &str,
    name: &str,
    columns: &[String],
    unique: bool,
) -> String {
    let u = if unique { "UNIQUE " } else { "" };
    let cols = columns
        .iter()
        .map(|c| ident(d, c))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "CREATE {u}INDEX {} ON {} ({cols});",
        ident(d, name),
        qualify(d, schema, table)
    )
}

pub fn drop_index(d: Dialect, schema: &str, table: &str, name: &str) -> String {
    match d {
        // MySQL indexes are scoped to their table.
        Dialect::MySql => format!(
            "DROP INDEX {} ON {};",
            ident(d, name),
            qualify(d, schema, table)
        ),
        Dialect::Sqlite => format!("DROP INDEX {};", ident(d, name)),
        // Postgres/SQL Server: the index lives in the table's schema.
        _ => format!("DROP INDEX {}.{};", ident(d, schema), ident(d, name)),
    }
}
