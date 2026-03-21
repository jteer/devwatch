//! Phase 3 — Tauri GUI client (not yet implemented).
//!
//! Implementation outline:
//!
//! 1. Spawn the `daemon` binary as a Tauri sidecar process using
//!    `tauri::api::process::Command` (configured in `tauri.conf.json`).
//! 2. Connect to the daemon via `tokio::net::TcpStream` on `127.0.0.1:{port}`,
//!    using newline-delimited JSON (same framing as the TUI client).
//! 3. Bridge daemon messages to the frontend via Tauri commands/events:
//!    - `#[tauri::command] fn get_state()` → returns current PR list as JSON
//!    - `app.emit_all("pr_event", payload)` for live updates
//! 4. System tray icon: `tauri::tray::TrayIcon`
//!    - Default icon when idle
//!    - Badge / alternate icon when unread PR events are present
//!    - Left-click → show/hide main window
//!    - Right-click → context menu (Open, Mark all read, Quit)

fn main() {
    // TODO: Phase 3 — implement Tauri GUI
    eprintln!("devwatch Tauri app is not yet implemented (Phase 3).");
    std::process::exit(1);
}
