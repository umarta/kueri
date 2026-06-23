//! PostgreSQL backup & restore by shelling out to the native client tools
//! (`pg_dump`, `pg_restore`, `psql`). The full connection config is passed from
//! the frontend so no extra state is needed; the password goes via `PGPASSWORD`
//! (never on the argv, never logged).

use tokio::process::Command;

use crate::db::connect::ConnectionConfig;
use crate::db::DbKind;
use crate::error::{AppError, AppResult};

fn ensure_pg(cfg: &ConnectionConfig) -> AppResult<()> {
    if cfg.kind != DbKind::Postgres {
        return Err(AppError::Other(
            "Backup & restore currently supports PostgreSQL only.".into(),
        ));
    }
    Ok(())
}

/// `format`: "plain" | "custom" · `contents`: "all" | "schema" | "data".
#[tauri::command]
pub async fn pg_export(
    cfg: ConnectionConfig,
    path: String,
    format: String,
    contents: String,
) -> AppResult<String> {
    ensure_pg(&cfg)?;
    let mut cmd = Command::new("pg_dump");
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
        .arg(&path);
    cmd.arg(if format == "custom" { "-Fc" } else { "-Fp" });
    match contents.as_str() {
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

#[tauri::command]
pub async fn pg_import(cfg: ConnectionConfig, path: String) -> AppResult<String> {
    ensure_pg(&cfg)?;
    let lower = path.to_lowercase();
    let custom =
        lower.ends_with(".dump") || lower.ends_with(".backup") || lower.ends_with(".pgdump");

    let (mut cmd, tool) = if custom {
        let mut c = Command::new("pg_restore");
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
            .arg(&path);
        (c, "pg_restore")
    } else {
        let mut c = Command::new("psql");
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
            .arg(&path);
        (c, "psql")
    };
    cmd.env("PGPASSWORD", &cfg.password);
    run(cmd, tool).await
}

async fn run(mut cmd: Command, tool: &str) -> AppResult<String> {
    let out = cmd.output().await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::Other(format!(
                "`{tool}` not found. Install the PostgreSQL client tools and make sure they're on your PATH."
            ))
        } else {
            AppError::Other(format!("{tool}: {e}"))
        }
    })?;
    if out.status.success() {
        Ok(format!("{tool} completed successfully."))
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        Err(AppError::Other(format!(
            "{tool} failed:\n{}",
            stderr.trim()
        )))
    }
}
