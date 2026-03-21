//! Daemon auto-launch: connect to the daemon, starting it first if needed.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use tokio::net::TcpStream;
use tracing::{info, warn};

/// Connect to the daemon at `127.0.0.1:port`.
///
/// If the connection is refused, the daemon binary is located and spawned
/// as a background process, then connection is retried with backoff.
/// Returns the connected `TcpStream` or an error with a clear message.
pub async fn connect_or_start(port: u16) -> Result<TcpStream> {
    let addr = format!("127.0.0.1:{port}");

    // Happy path: daemon is already running.
    match TcpStream::connect(&addr).await {
        Ok(stream) => {
            info!("connected to daemon at {addr}");
            return Ok(stream);
        }
        Err(e) if is_connection_refused(&e) => {
            info!("daemon not running, attempting to start it");
        }
        Err(e) => {
            return Err(e).with_context(|| format!("cannot connect to {addr}"));
        }
    }

    // Locate the daemon binary.
    let bin = daemon_binary_path()?;
    info!("spawning daemon from {}", bin.display());

    tokio::process::Command::new(&bin)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        // Detach from this process so it keeps running after the TUI exits.
        .kill_on_drop(false)
        .spawn()
        .with_context(|| format!("failed to spawn daemon at {}", bin.display()))?;

    // Retry with linear backoff for up to 3 seconds.
    let attempts = 15;
    let delay    = Duration::from_millis(200);

    for attempt in 1..=attempts {
        tokio::time::sleep(delay).await;
        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                info!("daemon ready after {attempt} attempt(s)");
                return Ok(stream);
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

/// Find the daemon binary:
/// 1. As a sibling of the current executable (installed / `cargo build` layout)
/// 2. Literally `"daemon"` — let the OS resolve it via PATH
fn daemon_binary_path() -> Result<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("daemon");
        if sibling.exists() {
            return Ok(sibling);
        }
    }
    // Fall back to PATH — will fail at spawn time with a clear OS error.
    Ok(PathBuf::from("daemon"))
}

fn is_connection_refused(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
    )
}
