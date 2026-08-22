//! Optional SSH tunnel for a connection. We shell out to the system `ssh`
//! client to open a local port-forward (`-L`), then point the driver at
//! `127.0.0.1:<local_port>`. Key/agent auth only (BatchMode — no prompts),
//! which covers the typical bastion setup without bundling an SSH stack.

use std::net::TcpListener;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};

use crate::db::connect::ConnectionConfigV2;
use crate::error::{AppError, AppResult};
use crate::ssh::profile::{SshAuth, SshRef};

fn free_port() -> AppResult<u16> {
    let l = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| AppError::Other(format!("no free local port: {e}")))?;
    l.local_addr()
        .map(|a| a.port())
        .map_err(|e| AppError::Other(e.to_string()))
}

/// Open a tunnel and return `(local_port, ssh_child)`. The child is configured
/// to be killed when dropped, so disconnecting (dropping it) tears the tunnel down.
pub async fn open(cfg: &ConnectionConfigV2) -> AppResult<(u16, Child)> {
    let ssh_ref = match &cfg.ssh {
        Some(r) => r,
        None => {
            return Err(AppError::Other(
                "open() called without an SSH config — this is a bug.".into(),
            ))
        }
    };

    let profile = match ssh_ref {
        SshRef::Inline(p) => p,
        SshRef::Profile(_) => {
            return Err(AppError::Other(
                "SSH profile references not yet supported (Phase 4).".into(),
            ))
        }
    };

    let lp = free_port()?;
    let ssh_port = profile.port;
    let db_host = &cfg.host;
    let fwd = format!("127.0.0.1:{lp}:{db_host}:{}", cfg.port);
    let target = format!("{}@{}", profile.user, profile.host);

    let mut cmd = Command::new("ssh");
    cmd.arg("-N")
        .arg("-T")
        .args(["-o", "BatchMode=yes"])
        .args(["-o", "ExitOnForwardFailure=yes"])
        .args(["-o", "StrictHostKeyChecking=accept-new"])
        .args(["-o", "ServerAliveInterval=30"])
        .arg("-p")
        .arg(ssh_port.to_string())
        .arg("-L")
        .arg(&fwd);

    match &profile.auth {
        SshAuth::Password { .. } => {
            return Err(AppError::Other(
                "SSH password auth is not supported in Phase 1. Use a key file or ssh-agent."
                    .into(),
            ));
        }
        SshAuth::KeyFile { path, .. } => {
            cmd.arg("-i").arg(path);
        }
        SshAuth::Agent => {
            // Relies on ssh-agent; no extra args needed.
        }
    }

    cmd.arg(&target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|e| {
        AppError::Other(format!(
            "could not start ssh (is the OpenSSH client installed?): {e}"
        ))
    })?;

    // Wait (up to ~6s) for the local forward to start accepting connections.
    for _ in 0..60 {
        if TcpStream::connect(("127.0.0.1", lp)).await.is_ok() {
            return Ok((lp, child));
        }
        if let Ok(Some(status)) = child.try_wait() {
            let mut err = String::new();
            if let Some(mut s) = child.stderr.take() {
                let _ = s.read_to_string(&mut err).await;
            }
            return Err(AppError::Other(format!(
                "SSH tunnel failed ({status}): {}",
                err.trim()
            )));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let _ = child.kill().await;
    Err(AppError::Other(
        "SSH tunnel did not become ready (check host/port/user/key and that key auth works)."
            .into(),
    ))
}
