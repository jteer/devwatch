use std::collections::VecDeque;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures_util::StreamExt;
use ratatui::widgets::TableState;
use tokio::sync::mpsc;

use devwatch_core::ipc::DaemonMessage;
use devwatch_core::types::{PullRequest, VcsEvent};
use tokio::process::Child;

const MAX_LOG_ENTRIES: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Disconnected,
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connecting    => write!(f, "Connecting…"),
            Self::Connected     => write!(f, "Connected"),
            Self::Disconnected  => write!(f, "Disconnected"),
        }
    }
}

pub struct LogEntry {
    pub timestamp: String,
    pub message: String,
}

pub struct App {
    pub prs: Vec<PullRequest>,
    pub log: VecDeque<LogEntry>,
    pub table_state: TableState,
    pub status: ConnectionStatus,
    pub should_quit: bool,
    /// When the last real VCS event (new/updated/closed PR) was received.
    pub last_event: Option<Instant>,
    /// Show "Polling…" in the status bar until this instant.
    /// Set to `now + 2s` on PollingStarted so even fast polls stay visible.
    pub polling_until: Option<Instant>,
    /// Child process handle when this TUI spawned the daemon.
    /// Kept alive here so `kill_on_drop(true)` fires on TUI exit.
    _daemon_child: Option<Child>,
    daemon_rx: mpsc::Receiver<DaemonMessage>,
}

impl App {
    pub fn new(
        daemon_rx: mpsc::Receiver<DaemonMessage>,
        daemon_child: Option<Child>,
    ) -> Self {
        Self {
            prs: Vec::new(),
            log: VecDeque::with_capacity(MAX_LOG_ENTRIES),
            table_state: TableState::default(),
            status: ConnectionStatus::Connecting,
            should_quit: false,
            last_event: None,
            polling_until: None,
            _daemon_child: daemon_child,
            daemon_rx,
        }
    }

    /// Seconds elapsed since the last real VCS event, or `None` if none received yet.
    pub fn event_timer(&self) -> Option<u64> {
        self.last_event.map(|t| t.elapsed().as_secs())
    }

    pub async fn run(mut self, mut terminal: ratatui::DefaultTerminal) -> Result<()> {
        let mut events = EventStream::new();
        // Drive redraws every second so the poll countdown ticks in real time.
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            terminal.draw(|frame| crate::ui::draw(frame, &mut self))?;

            tokio::select! {
                // 1-second tick — just triggers a redraw to update the timer.
                _ = tick.tick() => {}

                // Terminal key events
                event = events.next() => {
                    match event {
                        Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                            self.handle_key(key.code);
                        }
                        Some(Err(e)) => return Err(e.into()),
                        None => break,
                        _ => {}
                    }
                }

                // Daemon messages
                msg = self.daemon_rx.recv() => {
                    match msg {
                        Some(msg) => self.handle_daemon_msg(msg),
                        None => {
                            // Channel closed — daemon disconnected
                            self.status = ConnectionStatus::Disconnected;
                            self.push_log("daemon disconnected".to_string());
                        }
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Down | KeyCode::Char('j') => self.next_row(),
            KeyCode::Up   | KeyCode::Char('k') => self.prev_row(),
            KeyCode::Enter => self.open_selected_url(),
            _ => {}
        }
    }

    fn handle_daemon_msg(&mut self, msg: DaemonMessage) {
        match msg {
            DaemonMessage::PollingStarted => {
                // Keep "Polling…" visible for at least 2s so it's never missed.
                self.polling_until = Some(Instant::now() + Duration::from_secs(2));
            }
            DaemonMessage::PollingFinished => {}
            DaemonMessage::StateSnapshot { pull_requests } => {
                self.prs = pull_requests;
                self.status = ConnectionStatus::Connected;
                self.push_log(format!("snapshot: {} open PRs", self.prs.len()));
                // Select first row if nothing is selected yet.
                if !self.prs.is_empty() && self.table_state.selected().is_none() {
                    self.table_state.select(Some(0));
                }
            }
            DaemonMessage::Event(event) => {
                self.last_event = Some(Instant::now());
                self.handle_vcs_event(event);
            }
            DaemonMessage::Error { message } => {
                self.push_log(format!("error: {message}"));
            }
            DaemonMessage::Pong => {}
        }
    }

    fn handle_vcs_event(&mut self, event: VcsEvent) {
        match event {
            VcsEvent::NewPullRequest(pr) => {
                self.push_log(format!(
                    "new  PR #{} {}  [{}]",
                    pr.number, pr.title, pr.repo
                ));
                self.prs.push(pr);
                if self.table_state.selected().is_none() {
                    self.table_state.select(Some(0));
                }
            }
            VcsEvent::PullRequestUpdated { old: _, new } => {
                self.push_log(format!(
                    "upd  PR #{} {}  [{}]",
                    new.number, new.title, new.repo
                ));
                if let Some(pos) = self
                    .prs
                    .iter()
                    .position(|p| p.number == new.number && p.repo == new.repo)
                {
                    self.prs[pos] = new;
                }
            }
            VcsEvent::PullRequestClosed(pr) => {
                self.push_log(format!(
                    "closed PR #{} {}  [{}]",
                    pr.number, pr.title, pr.repo
                ));
                self.prs.retain(|p| !(p.number == pr.number && p.repo == pr.repo));
                // Keep selection in bounds.
                if let Some(sel) = self.table_state.selected() {
                    if !self.prs.is_empty() {
                        self.table_state.select(Some(sel.min(self.prs.len() - 1)));
                    } else {
                        self.table_state.select(None);
                    }
                }
            }
        }
    }

    fn next_row(&mut self) {
        if self.prs.is_empty() {
            return;
        }
        let next = match self.table_state.selected() {
            Some(i) => (i + 1).min(self.prs.len() - 1),
            None    => 0,
        };
        self.table_state.select(Some(next));
    }

    fn prev_row(&mut self) {
        if self.prs.is_empty() {
            return;
        }
        let prev = match self.table_state.selected() {
            Some(0) | None => 0,
            Some(i)        => i - 1,
        };
        self.table_state.select(Some(prev));
    }

    fn open_selected_url(&self) {
        if let Some(i) = self.table_state.selected() {
            if let Some(pr) = self.prs.get(i) {
                if !pr.url.is_empty() {
                    let _ = open::that(&pr.url);
                }
            }
        }
    }

    fn push_log(&mut self, message: String) {
        if self.log.len() == MAX_LOG_ENTRIES {
            self.log.pop_front();
        }
        self.log.push_back(LogEntry {
            timestamp: now_hms(),
            message,
        });
    }
}

fn now_hms() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}
