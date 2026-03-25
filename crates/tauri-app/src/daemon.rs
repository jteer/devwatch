//! Background task: connect to the devwatch daemon and bridge messages to the
//! Tauri frontend via events.

use std::sync::Mutex;
use std::time::Duration;

use anyhow::Result;
use futures_util::StreamExt;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::sleep;
use tokio_util::codec::{FramedRead, LinesCodec};

use devwatch_core::ipc::{ClientMessage, DaemonMessage};
use devwatch_core::types::VcsEvent;

use crate::{AppState, ConnectionStatus};

/// Entry point — runs forever, reconnecting on failure.
pub async fn run(app: AppHandle, port: u16) {
    loop {
        set_status(&app, ConnectionStatus::Connecting);
        match connect_and_read(&app, port).await {
            Ok(()) => {}
            Err(e) => eprintln!("[devwatch] daemon error: {e}"),
        }
        set_status(&app, ConnectionStatus::Disconnected);
        sleep(Duration::from_secs(3)).await;
    }
}

async fn connect_and_read(app: &AppHandle, port: u16) -> Result<()> {
    let stream = connect_or_start(port).await?;
    let (reader, mut writer) = stream.into_split();
    let mut framed = FramedRead::new(reader, LinesCodec::new());

    // Subscribe — daemon will send a StateSnapshot then live Events.
    let mut line = serde_json::to_string(&ClientMessage::Subscribe)?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;

    set_status(app, ConnectionStatus::Connected);

    while let Some(result) = framed.next().await {
        match result {
            Ok(line) => {
                if let Ok(msg) = serde_json::from_str::<DaemonMessage>(&line) {
                    handle_msg(app, msg);
                }
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn handle_msg(app: &AppHandle, msg: DaemonMessage) {
    let state = app.state::<Mutex<AppState>>();
    match msg {
        DaemonMessage::StateSnapshot { pull_requests } => {
            let mut s = state.lock().unwrap();
            s.prs = pull_requests;
            let _ = app.emit("pr-snapshot", s.prs.clone());
        }
        DaemonMessage::Event(event) => {
            {
                let mut s = state.lock().unwrap();
                match &event {
                    VcsEvent::NewPullRequest(pr) => {
                        s.prs.push(pr.clone());
                        s.unread += 1;
                    }
                    VcsEvent::PullRequestUpdated { new, .. } => {
                        if let Some(pos) = s
                            .prs
                            .iter()
                            .position(|p| p.number == new.number && p.repo == new.repo)
                        {
                            s.prs[pos] = new.clone();
                        }
                        s.unread += 1;
                    }
                    VcsEvent::PullRequestClosed(pr) => {
                        s.prs.retain(|p| !(p.number == pr.number && p.repo == pr.repo));
                    }
                }
                let _ = app.emit("pr-snapshot", s.prs.clone());
                let _ = app.emit("unread-count", s.unread);
            }
            let _ = app.emit("pr-event", serde_json::to_value(&event).ok());
        }
        DaemonMessage::PollingStarted  => { let _ = app.emit("polling", true);  }
        DaemonMessage::PollingFinished => { let _ = app.emit("polling", false); }
        DaemonMessage::Error { message } => { let _ = app.emit("daemon-error", message); }
        DaemonMessage::Pong => {}
    }
}

// ── Connection helpers ────────────────────────────────────────────────────────

async fn connect_or_start(port: u16) -> Result<TcpStream> {
    let addr = format!("127.0.0.1:{port}");

    match TcpStream::connect(&addr).await {
        Ok(s) => return Ok(s),
        Err(e) if is_refused(&e) => {}
        Err(e) => return Err(e.into()),
    }

    spawn_daemon()?;

    // Wait up to 30 s for the daemon to become ready.
    for _ in 0..150 {
        sleep(Duration::from_millis(200)).await;
        match TcpStream::connect(&addr).await {
            Ok(s) => return Ok(s),
            Err(e) if is_refused(&e) => continue,
            Err(e) => return Err(e.into()),
        }
    }
    anyhow::bail!("daemon did not start within 30 s — try running it manually: cargo run -p daemon")
}

fn spawn_daemon() -> Result<()> {
    let root = workspace_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    if is_workspace() {
        std::process::Command::new("cargo")
            .args(["run", "-p", "daemon"])
            .current_dir(&root)
            .spawn()?;
    } else {
        std::process::Command::new(sibling("daemon"))
            .current_dir(&root)
            .spawn()?;
    }
    Ok(())
}

/// Walk up from CWD to find the directory containing a `[workspace]` Cargo.toml.
fn workspace_root() -> Option<std::path::PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let c = dir.join("Cargo.toml");
        if c.exists() {
            if let Ok(s) = std::fs::read_to_string(&c) {
                if s.contains("[workspace]") {
                    return Some(dir);
                }
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn is_workspace() -> bool {
    let Ok(mut dir) = std::env::current_dir() else { return false };
    loop {
        let c = dir.join("Cargo.toml");
        if c.exists() {
            if let Ok(s) = std::fs::read_to_string(&c) {
                if s.contains("[workspace]") { return true; }
            }
        }
        if !dir.pop() { return false; }
    }
}

fn sibling(name: &str) -> std::path::PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let s = exe.parent().unwrap_or(std::path::Path::new(".")).join(name);
        if s.exists() { return s; }
    }
    std::path::PathBuf::from(name)
}

fn is_refused(e: &std::io::Error) -> bool {
    matches!(e.kind(), std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound)
}

fn set_status(app: &AppHandle, status: ConnectionStatus) {
    let label = match &status {
        ConnectionStatus::Connected    => "connected",
        ConnectionStatus::Connecting   => "connecting",
        ConnectionStatus::Disconnected => "disconnected",
    };
    {
        let state = app.state::<Mutex<AppState>>();
        state.lock().unwrap().status = status;
    }
    let _ = app.emit("connection-status", label);
}
