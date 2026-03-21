//! Phase 2 — ratatui TUI client (not yet implemented).
//!
//! Implementation outline:
//!
//! 1. Connect via `tokio::net::TcpStream` to `127.0.0.1:{daemon_port}`.
//! 2. Use `tokio_util::codec::LinesCodec` for newline-delimited JSON framing
//!    (same as the daemon server — no extra protocol negotiation).
//! 3. Send `ClientMessage::Subscribe` → receive `DaemonMessage::StateSnapshot`,
//!    then stream `DaemonMessage::Event` messages.
//! 4. Initialise ratatui: `ratatui::init()` / `ratatui::restore()`.
//! 5. Main loop via `tokio::select!` between:
//!    - `crossterm::event::EventStream` for keyboard input
//!    - daemon message channel for live PR updates
//! 6. Layout:
//!    - Top pane:    PR list using `ratatui::widgets::Table`
//!    - Bottom pane: Recent event log using `ratatui::widgets::List`
//! 7. Key bindings: `q` to quit, `↑`/`↓` to navigate, `Enter` to open PR URL.

fn main() {
    // TODO: Phase 2 — implement TUI client
    eprintln!("devwatch TUI is not yet implemented (Phase 2).");
    std::process::exit(1);
}
