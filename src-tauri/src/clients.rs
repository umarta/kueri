//! PostgreSQL client-tool discovery & install (the TablePlus approach).
//!
//! `pg_dump` refuses to dump a server newer than itself, so for the binary
//! Custom (.dump) export we want a `pg_dump` whose major matches the server.
//! We scan the usual install locations, report every version found, and (on
//! macOS via Homebrew) can install a matching one on demand.

use serde::Serialize;
use tokio::process::Command;

use crate::error::{AppError, AppResult};

#[derive(Serialize)]
pub struct ClientTool {
    /// Major version, e.g. "17".
    pub major: String,
    /// Full version, e.g. "17.10".
    pub full: String,
    /// Folder holding pg_dump (empty = found on PATH).
    pub bin: String,
}

/// Directories that commonly hold a PostgreSQL `bin/`.
fn candidate_bins() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let home = std::env::var("HOME").unwrap_or_default();

    let push_children = |out: &mut Vec<String>, base: &str, child: &str| {
        if let Ok(rd) = std::fs::read_dir(base) {
            for e in rd.flatten() {
                let p = e.path().join(child);
                if p.is_dir() {
                    out.push(p.to_string_lossy().into_owned());
                }
            }
        }
    };

    // Homebrew (arm + intel): /opt/homebrew/opt/postgresql@17/bin, …
    push_children(&mut out, "/opt/homebrew/opt", "bin");
    push_children(&mut out, "/usr/local/opt", "bin");
    // Postgres.app: /Applications/Postgres.app/Contents/Versions/17/bin
    push_children(
        &mut out,
        "/Applications/Postgres.app/Contents/Versions",
        "bin",
    );
    // EDB installer: /Library/PostgreSQL/17/bin
    push_children(&mut out, "/Library/PostgreSQL", "bin");
    // DBngin engines
    push_children(
        &mut out,
        &format!("{home}/Library/Application Support/com.tinyapp.DBngin/Engines/postgresql"),
        "bin",
    );
    // Linux: /usr/lib/postgresql/17/bin, /usr/pgsql-17/bin
    push_children(&mut out, "/usr/lib/postgresql", "bin");
    if let Ok(rd) = std::fs::read_dir("/usr") {
        for e in rd.flatten() {
            let name = e.file_name();
            if name.to_string_lossy().starts_with("pgsql") {
                let p = e.path().join("bin");
                if p.is_dir() {
                    out.push(p.to_string_lossy().into_owned());
                }
            }
        }
    }
    // Keep only dirs that actually contain pg_dump.
    out.retain(|d| std::path::Path::new(d).join("pg_dump").exists());
    out
}

async fn pg_dump_version(bin_dir: &str) -> Option<(String, String)> {
    let exe = if bin_dir.is_empty() {
        "pg_dump".to_string()
    } else {
        format!("{bin_dir}/pg_dump")
    };
    let out = Command::new(&exe).arg("--version").output().await.ok()?;
    if !out.status.success() {
        return None;
    }
    // "pg_dump (PostgreSQL) 17.10 (Homebrew)"
    let s = String::from_utf8_lossy(&out.stdout);
    let token = s
        .split_whitespace()
        .find(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit()) && t.contains('.'))?;
    let full = token.trim().to_string();
    let major = full.split('.').next().unwrap_or(&full).to_string();
    Some((major, full))
}

/// Every distinct `pg_dump` major version found on this machine.
#[tauri::command]
pub async fn detect_clients() -> AppResult<Vec<ClientTool>> {
    let mut dirs = candidate_bins();
    dirs.push(String::new()); // also probe whatever is on PATH

    let mut out: Vec<ClientTool> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for dir in dirs {
        if let Some((major, full)) = pg_dump_version(&dir).await {
            if seen.insert(major.clone()) {
                out.push(ClientTool {
                    major,
                    full,
                    bin: dir,
                });
            }
        }
    }
    out.sort_by(|a, b| {
        b.major
            .parse::<u32>()
            .unwrap_or(0)
            .cmp(&a.major.parse::<u32>().unwrap_or(0))
    });
    Ok(out)
}

fn which_brew() -> Option<String> {
    ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
        .into_iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(String::from)
}

/// Install a matching client. macOS: `brew install postgresql@<major>`.
/// Returns the bin dir to use on success.
#[tauri::command]
pub async fn install_pg_client(major: String) -> AppResult<String> {
    if !cfg!(target_os = "macos") {
        return Err(AppError::Other(
            "Auto-install is wired for macOS (Homebrew). Download PostgreSQL from https://www.postgresql.org/download/".into(),
        ));
    }
    let brew = which_brew().ok_or_else(|| {
        AppError::Other(
            "Homebrew not found. Install it from https://brew.sh, or download PostgreSQL from https://www.postgresql.org/download/".into(),
        )
    })?;
    let out = Command::new(&brew)
        .arg("install")
        .arg(format!("postgresql@{major}"))
        .output()
        .await
        .map_err(|e| AppError::Other(format!("brew: {e}")))?;
    if !out.status.success() {
        return Err(AppError::Other(format!(
            "brew install postgresql@{major} failed:\n{}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    // Homebrew keg location.
    for base in ["/opt/homebrew/opt", "/usr/local/opt"] {
        let bin = format!("{base}/postgresql@{major}/bin");
        if std::path::Path::new(&bin).join("pg_dump").exists() {
            return Ok(bin);
        }
    }
    Ok(String::new())
}

/// Open a URL in the default browser (download pages).
#[tauri::command]
pub fn open_url(url: String) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    let (cmd, args): (&str, Vec<&str>) = ("open", vec![&url]);
    #[cfg(target_os = "windows")]
    let (cmd, args): (&str, Vec<&str>) = ("cmd", vec!["/C", "start", "", &url]);
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let (cmd, args): (&str, Vec<&str>) = ("xdg-open", vec![&url]);
    std::process::Command::new(cmd)
        .args(args)
        .spawn()
        .map_err(|e| AppError::Other(format!("open url: {e}")))?;
    Ok(())
}
