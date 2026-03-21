mod app;
mod ui;

use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, LinesCodec};
use futures_util::StreamExt;

use devwatch_core::ipc::{ClientMessage, DaemonMessage};

#[tokio::main]
async fn main() -> Result<()> {
    // ── Logging (to file so it doesn't corrupt the terminal) ─────────────────
    // Tracing output is suppressed in TUI mode; set DEVWATCH_TUI_LOG to a file
    // path to enable: DEVWATCH_TUI_LOG=/tmp/tui.log devwatch-tui
    if let Ok(log_path) = std::env::var("DEVWATCH_TUI_LOG") {
        let file = std::fs::File::create(&log_path)
            .with_context(|| format!("create log file {log_path}"))?;
        tracing_subscriber::fmt()
            .with_writer(std::sync::Mutex::new(file))
            .with_ansi(false)
            .init();
    }

    // ── Config ────────────────────────────────────────────────────────────────
    let port = load_port();

    // ── Connect to daemon ─────────────────────────────────────────────────────
    let addr = format!("127.0.0.1:{port}");
    let stream = TcpStream::connect(&addr)
        .await
        .with_context(|| format!("cannot connect to daemon at {addr} — is it running?"))?;

    let (reader, mut writer) = stream.into_split();
    let mut framed_reader = FramedRead::new(reader, LinesCodec::new());

    // Subscribe immediately so the daemon queues events from the start.
    let mut subscribe_line = serde_json::to_string(&ClientMessage::Subscribe)?;
    subscribe_line.push('\n');
    writer
        .write_all(subscribe_line.as_bytes())
        .await
        .context("failed to send Subscribe")?;

    // ── Daemon reader task ────────────────────────────────────────────────────
    let (daemon_tx, daemon_rx) = mpsc::channel::<DaemonMessage>(64);

    tokio::spawn(async move {
        while let Some(result) = framed_reader.next().await {
            match result {
                Ok(line) => {
                    if let Ok(msg) = serde_json::from_str::<DaemonMessage>(&line) {
                        if daemon_tx.send(msg).await.is_err() {
                            break; // TUI exited
                        }
                    }
                }
                Err(_) => break, // daemon closed
            }
        }
    });

    // ── Run TUI ───────────────────────────────────────────────────────────────
    let terminal = ratatui::init();
    let result = app::App::new(daemon_rx).run(terminal).await;
    ratatui::restore();

    result
}

fn load_port() -> u16 {
    config::Config::builder()
        .add_source(config::File::with_name("config").required(false))
        .add_source(config::Environment::with_prefix("DEVWATCH").separator("__"))
        .build()
        .and_then(|c| c.get_int("daemon_port"))
        .map(|p| p as u16)
        .unwrap_or(7878)
}
