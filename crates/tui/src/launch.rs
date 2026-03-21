//! Daemon auto-launch: connect to the daemon, starting it first if needed.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use tokio::net::TcpStream;
use tokio::process::Child;
use tracing::{info, warn};

/// Result of connecting to the daemon.
pub struct Connection {
    pub stream: TcpStream,
    /// Present only when *this* TUI instance spawned the daemon.
    /// Held with `kill_on_drop(true)` so the daemon is automatically
    /// terminated when the TUI exits (the `App` drops this value).
    pub owned_child: Option<Child>,
}

/// Connect to the daemon at `127.0.0.1:port`.
///
/// If the connection is refused, the daemon binary is located and spawned.
/// Returns a `Connection` whose `owned_child` is `Some` only when we started
/// the daemon — callers should keep it alive for the duration of the session.
pub async fn connect_or_start(port: u16) -> Result<Connection> {
    let addr = format!("127.0.0.1:{port}");

    // Happy path: daemon is already running.
    match TcpStream::connect(&addr).await {
        Ok(stream) => {
            info!("connected to existing daemon at {addr}");
            return Ok(Connection { stream, owned_child: None });
        }
        Err(e) if is_connection_refused(&e) => {
            info!("daemon not running, attempting to start it");
        }
        Err(e) => {
            return Err(e).with_context(|| format!("cannot connect to {addr}"));
        }
    }

    // Locate and spawn the daemon. In a Cargo workspace (dev) we use
    // `cargo run -p daemon` so the binary is always rebuilt before launch.
    // In a release/installed layout we run the sibling binary directly.
    let child = spawn_daemon()?;

    // Retry with linear backoff for up to 3 seconds.
    let attempts = 15;
    let delay    = Duration::from_millis(200);

    for attempt in 1..=attempts {
        tokio::time::sleep(delay).await;
        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                info!("daemon ready after {attempt} attempt(s)");
                return Ok(Connection { stream, owned_child: Some(child) });
            }
            Err(e) if is_connection_refused(&e) => {
                warn!("attempt {attempt}/{attempts}: daemon not ready yet");
            }
            Err(e) => {
                return Err(e).with_context(|| format!("unexpected error connecting to {addr}"));
            }
        }
    }

    Err(anyhow!(
        "daemon did not become ready within {}s\n\
         Try running it manually: cargo run -p daemon",
        attempts as f32 * delay.as_secs_f32()
    ))
}

/// Spawn the daemon process and return the `Child` handle.
///
/// Strategy:
/// 1. If we can find a `Cargo.toml` with `[workspace]` by walking up from the
///    current working directory, we are in a dev/workspace layout — run
///    `cargo run -p daemon` so the daemon binary is always freshly built.
/// 2. Otherwise (installed layout) run the sibling binary directly.
fn spawn_daemon() -> Result<Child> {
    let mut cmd = if is_cargo_workspace() {
        info!("workspace detected — spawning daemon via `cargo run -p daemon`");
        let mut c = tokio::process::Command::new("cargo");
        c.args(["run", "-p", "daemon"]);
        c
    } else {
        let path = daemon_binary_path();
        info!("spawning daemon binary at {}", path.display());
        tokio::process::Command::new(path)
    };

    cmd.kill_on_drop(true)
        .spawn()
        .context("failed to spawn daemon process")
}

/// Walk up from the current working directory looking for a `Cargo.toml`
/// that contains `[workspace]`.  Returns `true` when found.
fn is_cargo_workspace() -> bool {
    let Ok(mut dir) = std::env::current_dir() else { return false };
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            if let Ok(contents) = std::fs::read_to_string(&candidate) {
                if contents.contains("[workspace]") {
                    return true;
                }
            }
        }
        if !dir.pop() {
            return false;
        }
    }
}

/// Find the daemon binary sibling of the current executable.
fn daemon_binary_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("daemon");
        if sibling.exists() {
            return sibling;
        }
    }
    // Fall back to PATH — will fail at spawn time with a clear OS error.
    PathBuf::from("daemon")
}

fn is_connection_refused(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
    )
}
