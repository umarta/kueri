//! Database backup & restore by shelling out to the native client tools
//! (`pg_dump`/`pg_restore`/`psql`, `mysqldump`/`mysql`) or — for SQLite — a file
//! copy. The full connection config is passed from the frontend; passwords go
//! via env (`PGPASSWORD`/`MYSQL_PWD`), never on the argv, never logged.

use std::process::Stdio;
use tokio::process::Command;

use crate::db::connect::ConnectionConfig;
use crate::db::DbKind;
use crate::error::{AppError, AppResult};

fn sqlite_file(cfg: &ConnectionConfig) -> String {
    cfg.file_path
        .clone()
        .unwrap_or_else(|| cfg.database.clone())
}

/// Resolve a client-tool binary, optionally from a user-set folder.
fn bin(tools: &str, name: &str) -> String {
    let t = tools.trim().trim_end_matches('/');
    if t.is_empty() {
        name.into()
    } else {
        format!("{t}/{name}")
    }
}

/// `format`: "plain" | "custom" · `contents`: "all" | "schema" | "data".
/// `tools`: optional folder for pg_dump/mysqldump (empty = PATH).
#[tauri::command]
pub async fn pg_export(
    cfg: ConnectionConfig,
    path: String,
    format: String,
    contents: String,
    tools: String,
) -> AppResult<String> {
    match cfg.kind {
        DbKind::Postgres => pg_dump(&cfg, &path, &format, &contents, &tools).await,
        DbKind::Mysql => mysqldump(&cfg, &path, &contents, &tools).await,
        DbKind::Sqlite => {
            std::fs::copy(sqlite_file(&cfg), &path)
                .map_err(|e| AppError::Other(format!("copy database file: {e}")))?;
            Ok("SQLite database file copied.".into())
        }
        _ => Err(AppError::Other(
            "Backup & restore is supported for PostgreSQL, MySQL and SQLite.".into(),
        )),
    }
}

async fn pg_dump(
    cfg: &ConnectionConfig,
    path: &str,
    format: &str,
    contents: &str,
    tools: &str,
) -> AppResult<String> {
    let mut cmd = Command::new(bin(tools, "pg_dump"));
    cmd.env("PGPASSWORD", &cfg.password)
        .arg("-h")
        .arg(&cfg.host)
        .arg("-p")
        .arg(cfg.port.to_string())
        .arg("-U")
        .arg(&cfg.user)
        .arg("-d")
        .arg(&cfg.database)
        .arg("-f")
        .arg(path);
    cmd.arg(if format == "custom" { "-Fc" } else { "-Fp" });
    match contents {
        "schema" => {
            cmd.arg("--schema-only");
        }
        "data" => {
            cmd.arg("--data-only");
        }
        _ => {}
    }
    run(cmd, "pg_dump").await
}

async fn mysqldump(
    cfg: &ConnectionConfig,
    path: &str,
    contents: &str,
    tools: &str,
) -> AppResult<String> {
    let mut cmd = Command::new(bin(tools, "mysqldump"));
    cmd.env("MYSQL_PWD", &cfg.password)
        .arg("-h")
        .arg(&cfg.host)
        .arg("-P")
        .arg(cfg.port.to_string())
        .arg("-u")
        .arg(&cfg.user)
        .arg("--result-file")
        .arg(path);
    match contents {
        "schema" => {
            cmd.arg("--no-data");
        }
        "data" => {
            cmd.arg("--no-create-info");
        }
        _ => {}
    }
    cmd.arg(&cfg.database);
    run(cmd, "mysqldump").await
}

#[tauri::command]
pub async fn pg_import(cfg: ConnectionConfig, path: String, tools: String) -> AppResult<String> {
    match cfg.kind {
        DbKind::Postgres => pg_restore_or_psql(&cfg, &path, &tools).await,
        DbKind::Mysql => mysql_restore(&cfg, &path, &tools).await,
        DbKind::Sqlite => {
            std::fs::copy(&path, sqlite_file(&cfg))
                .map_err(|e| AppError::Other(format!("restore database file: {e}")))?;
            Ok("SQLite database file restored — reconnect to see the changes.".into())
        }
        _ => Err(AppError::Other(
            "Backup & restore is supported for PostgreSQL, MySQL and SQLite.".into(),
        )),
    }
}

async fn mysql_restore(cfg: &ConnectionConfig, path: &str, tools: &str) -> AppResult<String> {
    let file =
        std::fs::File::open(path).map_err(|e| AppError::Other(format!("open {path}: {e}")))?;
    let mut cmd = Command::new(bin(tools, "mysql"));
    cmd.env("MYSQL_PWD", &cfg.password)
        .arg("-h")
        .arg(&cfg.host)
        .arg("-P")
        .arg(cfg.port.to_string())
        .arg("-u")
        .arg(&cfg.user)
        .arg(&cfg.database)
        .stdin(Stdio::from(file));
    run(cmd, "mysql").await
}

async fn pg_restore_or_psql(cfg: &ConnectionConfig, path: &str, tools: &str) -> AppResult<String> {
    let lower = path.to_lowercase();
    let custom =
        lower.ends_with(".dump") || lower.ends_with(".backup") || lower.ends_with(".pgdump");

    let (mut cmd, tool) = if custom {
        let mut c = Command::new(bin(tools, "pg_restore"));
        c.arg("--no-owner")
            .arg("--clean")
            .arg("--if-exists")
            .arg("-h")
            .arg(&cfg.host)
            .arg("-p")
            .arg(cfg.port.to_string())
            .arg("-U")
            .arg(&cfg.user)
            .arg("-d")
            .arg(&cfg.database)
            .arg(path);
        (c, "pg_restore")
    } else {
        let mut c = Command::new(bin(tools, "psql"));
        c.arg("-h")
            .arg(&cfg.host)
            .arg("-p")
            .arg(cfg.port.to_string())
            .arg("-U")
            .arg(&cfg.user)
            .arg("-d")
            .arg(&cfg.database)
            .arg("-v")
            .arg("ON_ERROR_STOP=1")
            .arg("-f")
            .arg(path);
        (c, "psql")
    };
    cmd.env("PGPASSWORD", &cfg.password);
    run(cmd, tool).await
}

async fn run(mut cmd: Command, tool: &str) -> AppResult<String> {
    let out = cmd.output().await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::Other(format!(
                "`{tool}` not found. Install the client tools, or set their folder in Settings → Client tools folder."
            ))
        } else {
            AppError::Other(format!("{tool}: {e}"))
        }
    })?;
    if out.status.success() {
        Ok(format!("{tool} completed successfully."))
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let hint = if stderr.contains("version mismatch") {
            "\n\nYour client tools are older than the server. Point Settings → Client tools folder at a matching version (e.g. the server's bin directory)."
        } else {
            ""
        };
        Err(AppError::Other(format!(
            "{tool} failed:\n{}{hint}",
            stderr.trim()
        )))
    }
}
