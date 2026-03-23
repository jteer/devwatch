mod app;
mod config_editor;
mod launch;
mod settings;
mod ui;

use std::path::PathBuf;

use anyhow::Result;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, LinesCodec};

use devwatch_core::config::AppConfig;
use devwatch_core::ipc::{ClientMessage, DaemonMessage};

#[tokio::main]
async fn main() -> Result<()> {
    // ── Logging (to file so it doesn't corrupt the terminal) ─────────────────
    if let Ok(log_path) = std::env::var("DEVWATCH_TUI_LOG") {
        let file = std::fs::File::create(&log_path)?;
        tracing_subscriber::fmt()
            .with_writer(std::sync::Mutex::new(file))
            .with_ansi(false)
            .init();
    }

    let demo_mode = std::env::args().any(|a| a == "--demo");

    // ── Config ────────────────────────────────────────────────────────────────
    let (cfg, config_path) = load_cfg_with_path();

    // ── Demo mode: skip daemon entirely ───────────────────────────────────────
    if demo_mode {
        let terminal = ratatui::init();
        let mut app = app::App::demo(cfg, config_path);
        if let Some(order) = settings::load_column_order() {
            app.column_order = order;
        }
        let result = app.run(terminal).await;
        ratatui::restore();
        return result;
    }

    // ── Connect (auto-starting daemon if not running) ─────────────────────────
    let port = cfg.daemon_port;
    let conn = launch::connect_or_start(port).await?;

    let (reader, mut writer) = conn.stream.into_split();
    let mut framed_reader = FramedRead::new(reader, LinesCodec::new());

    // Subscribe immediately so the daemon queues events from the start.
    let mut subscribe_line = serde_json::to_string(&ClientMessage::Subscribe)?;
    subscribe_line.push('\n');
    writer.write_all(subscribe_line.as_bytes()).await?;

    // ── Daemon reader task ────────────────────────────────────────────────────
    let (daemon_tx, daemon_rx) = mpsc::channel::<DaemonMessage>(64);

    tokio::spawn(async move {
        while let Some(result) = framed_reader.next().await {
            match result {
                Ok(line) => {
                    if let Ok(msg) = serde_json::from_str::<DaemonMessage>(&line) {
                        if daemon_tx.send(msg).await.is_err() {
                            break;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    // ── Run TUI ───────────────────────────────────────────────────────────────
    let terminal = ratatui::init();
    let mut app = app::App::new(daemon_rx, conn.owned_child, cfg, config_path);
    if let Some(order) = settings::load_column_order() {
        app.column_order = order;
    }
    let result = app.run(terminal).await;
    ratatui::restore();

    result
}

/// Returns the loaded config and the path it was / should be written to.
fn load_cfg_with_path() -> (AppConfig, PathBuf) {
    let config_path = PathBuf::from("config.toml");
    let cfg = config::Config::builder()
        .add_source(config::File::with_name("config").required(false))
        .add_source(config::Environment::with_prefix("DEVWATCH").separator("__"))
        .build()
        .and_then(|c| c.try_deserialize::<AppConfig>())
        .unwrap_or_else(|_| AppConfig {
            daemon_port: 7878,
            poll_interval_secs: 60,
            repos: vec![],
        });
    (cfg, config_path)
}
