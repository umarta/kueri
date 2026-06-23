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

use crate::db::connect::ConnectionConfig;
use crate::error::{AppError, AppResult};

fn free_port() -> AppResult<u16> {
    let l = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| AppError::Other(format!("no free local port: {e}")))?;
    l.local_addr()
        .map(|a| a.port())
        .map_err(|e| AppError::Other(e.to_string()))
}

/// Open a tunnel and return `(local_port, ssh_child)`. The child is configured
/// to be killed when dropped, so disconnecting (dropping it) tears the tunnel down.
pub async fn open(cfg: &ConnectionConfig) -> AppResult<(u16, Child)> {
    let lp = free_port()?;
    let ssh_port = if cfg.ssh_port == 0 { 22 } else { cfg.ssh_port };
    let db_host = if cfg.host.is_empty() {
        "127.0.0.1"
    } else {
        cfg.host.as_str()
    };
    let target = format!("{}@{}", cfg.ssh_user, cfg.ssh_host);
    let fwd = format!("127.0.0.1:{lp}:{db_host}:{}", cfg.port);

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
    if let Some(key) = cfg
        .ssh_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        cmd.arg("-i").arg(key);
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
