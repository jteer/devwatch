use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures_util::StreamExt;
use ratatui::widgets::TableState;
use tokio::sync::mpsc;

use devwatch_core::config::AppConfig;
use devwatch_core::ipc::DaemonMessage;
use devwatch_core::types::{PullRequest, VcsEvent};
use tokio::process::Child;

use crate::config_editor::{ConfigAction, ConfigEditor};

const MAX_LOG_ENTRIES: usize = 100;

// ── Connection status ─────────────────────────────────────────────────────────

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

// ── Column identity ───────────────────────────────────────────────────────────

/// Each column in the PR table, used to drive dynamic ordering.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ColumnId {
    Number,
    Repo,
    Title,
    Author,
    Age,
    State,
}

impl ColumnId {
    pub fn header(self) -> &'static str {
        match self {
            Self::Number => "  #",
            Self::Repo   => "Repo",
            Self::Title  => "Title",
            Self::Author => "Author",
            Self::Age    => "Age",
            Self::State  => "State",
        }
    }

    pub fn default_order() -> Vec<Self> {
        vec![Self::Number, Self::Repo, Self::Title, Self::Author, Self::Age, Self::State]
    }
}

// ── App mode ──────────────────────────────────────────────────────────────────

pub enum AppMode {
    /// Normal PR-list view.
    Normal,
    /// Config editor overlay.
    Config(ConfigEditor),
    /// Column reorder mode — `cursor` is the index of the highlighted column.
    ReorderColumns { cursor: usize },
}

// ── Log entry ─────────────────────────────────────────────────────────────────

pub struct LogEntry {
    pub timestamp: String,
    pub message: String,
}

// ── App ───────────────────────────────────────────────────────────────────────

pub struct App {
    pub prs: Vec<PullRequest>,
    pub log: VecDeque<LogEntry>,
    pub table_state: TableState,
    pub status: ConnectionStatus,
    pub should_quit: bool,
    /// When the last real VCS event (new/updated/closed PR) was received.
    pub last_event: Option<Instant>,
    /// Show "Polling…" in the status bar until this instant.
    pub polling_until: Option<Instant>,
    /// Current UI mode.
    pub mode: AppMode,
    /// Ordered list of visible columns (user-reorderable).
    pub column_order: Vec<ColumnId>,
    /// True when running in `--demo` mode (no daemon).
    pub is_demo: bool,
    /// Path to config file, used by the config editor.
    pub config_path: PathBuf,
    /// Loaded config (for opening the editor pre-populated).
    pub config: AppConfig,
    /// Child process handle when this TUI spawned the daemon.
    _daemon_child: Option<Child>,
    /// Keeps the demo channel open; `None` in normal mode.
    _demo_keep_alive: Option<mpsc::Sender<DaemonMessage>>,
    daemon_rx: mpsc::Receiver<DaemonMessage>,
}

impl App {
    pub fn new(
        daemon_rx: mpsc::Receiver<DaemonMessage>,
        daemon_child: Option<Child>,
        config: AppConfig,
        config_path: PathBuf,
    ) -> Self {
        Self {
            prs: Vec::new(),
            log: VecDeque::with_capacity(MAX_LOG_ENTRIES),
            table_state: TableState::default(),
            status: ConnectionStatus::Connecting,
            should_quit: false,
            last_event: None,
            polling_until: None,
            mode: AppMode::Normal,
            column_order: ColumnId::default_order(),
            is_demo: false,
            config_path,
            config,
            _daemon_child: daemon_child,
            _demo_keep_alive: None,
            daemon_rx,
        }
    }

    /// Create a pre-populated demo instance (no real daemon needed).
    pub fn demo(config: AppConfig, config_path: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel::<DaemonMessage>(1);
        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let prs = vec![
            pr( 1, "owner/frontend", "Add dark mode to settings panel",          "alice",   "open", now_unix - 60 * 25,    false),
            pr( 2, "owner/backend",  "Fix memory leak in connection pool",        "bob",     "open", now_unix - 3600 * 2,   false),
            pr( 3, "owner/frontend", "WIP: Refactor navigation sidebar",          "charlie", "open", now_unix - 3600,       true),
            pr( 4, "owner/api",      "Update rate limiting middleware",            "dave",    "open", now_unix - 3600 * 18,  false),
            pr( 5, "owner/backend",  "Bump all dependencies to latest",           "eve",     "open", now_unix - 86400 * 5,  false),
            pr( 6, "owner/api",      "Add OpenAPI spec for /v2 routes",           "frank",   "open", now_unix - 86400 * 21, false),
            pr( 7, "owner/infra",    "Migrate CI to GitHub Actions",              "grace",   "open", now_unix - 3600 * 6,   false),
            pr( 8, "owner/frontend", "WIP: Add i18n support",                     "heidi",   "open", now_unix - 60 * 45,    true),
            pr( 9, "owner/backend",  "Optimize slow queries in reports endpoint", "ivan",    "open", now_unix - 86400 * 3,  false),
            pr(10, "owner/api",      "Deprecate v1 authentication endpoints",     "judy",    "open", now_unix - 86400 * 12, false),
            pr(11, "owner/infra",    "Add Terraform modules for staging env",     "karl",    "open", now_unix - 3600 * 30,  false),
            pr(12, "owner/backend",  "Replace Redis cache with in-process LRU",   "laura",   "open", now_unix - 86400 * 2,  false),
            pr(13, "owner/frontend", "Fix mobile layout on small screens",        "mallory", "open", now_unix - 60 * 10,    false),
            pr(14, "owner/api",      "Add webhook support for PR events",         "niaj",    "open", now_unix - 86400 * 8,  false),
            pr(15, "owner/infra",    "Upgrade Kubernetes cluster to 1.30",        "oscar",   "open", now_unix - 86400 * 15, false),
            pr(16, "owner/backend",  "Add structured logging with tracing crate", "peggy",   "open", now_unix - 3600 * 4,   false),
        ];

        let mut log: VecDeque<LogEntry> = VecDeque::with_capacity(MAX_LOG_ENTRIES);
        let t = |offset: i64| {
            let s = now_unix as i64 - offset;
            let h = (s / 3600) % 24;
            let m = (s / 60)   % 60;
            let sc = s % 60;
            format!("{h:02}:{m:02}:{sc:02}")
        };
        log.push_back(LogEntry { timestamp: t(3),   message: "new  PR #16 Add structured logging with tracing crate  [owner/backend]".into() });
        log.push_back(LogEntry { timestamp: t(10),  message: "new  PR #13 Fix mobile layout on small screens  [owner/frontend]".into() });
        log.push_back(LogEntry { timestamp: t(25),  message: "upd  PR #4 Update rate limiting middleware  [owner/api]".into() });
        log.push_back(LogEntry { timestamp: t(60),  message: "closed PR #0 Remove legacy auth flow  [owner/backend]".into() });
        log.push_back(LogEntry { timestamp: t(120), message: "snapshot: 16 open PRs".into() });

        let mut app = Self {
            prs,
            log,
            table_state: TableState::default().with_selected(Some(0)),
            status: ConnectionStatus::Connected,
            should_quit: false,
            last_event: Some(Instant::now() - Duration::from_secs(3)),
            polling_until: None,
            mode: AppMode::Normal,
            column_order: ColumnId::default_order(),
            is_demo: true,
            config_path,
            config,
            _daemon_child: None,
            _demo_keep_alive: Some(tx),
            daemon_rx: rx,
        };
        app.push_log("demo mode — no daemon connection".to_string());
        app
    }

    /// Seconds elapsed since the last real VCS event, or `None` if none received yet.
    pub fn event_timer(&self) -> Option<u64> {
        self.last_event.map(|t| t.elapsed().as_secs())
    }

    pub async fn run(mut self, mut terminal: ratatui::DefaultTerminal) -> Result<()> {
        let mut events = EventStream::new();
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Demo: pulse "Polling…" every 8 seconds to show the indicator.
        let mut demo_tick = tokio::time::interval(Duration::from_secs(8));
        demo_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            terminal.draw(|frame| crate::ui::draw(frame, &mut self))?;

            tokio::select! {
                _ = tick.tick() => {}

                // Demo polling pulse (guarded — never fires in normal mode)
                _ = demo_tick.tick(), if self.is_demo => {
                    self.polling_until = Some(Instant::now() + Duration::from_secs(2));
                }

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

                msg = self.daemon_rx.recv() => {
                    match msg {
                        Some(msg) => self.handle_daemon_msg(msg),
                        None if !self.is_demo => {
                            self.status = ConnectionStatus::Disconnected;
                            self.push_log("daemon disconnected".to_string());
                        }
                        _ => {}
                    }
                }
            }

            if self.should_quit { break; }
        }
        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode) {
        match &mut self.mode {
            AppMode::Config(editor) => {
                match editor.handle_key(code) {
                    ConfigAction::Exit => self.mode = AppMode::Normal,
                    ConfigAction::None => {}
                }
            }
            AppMode::ReorderColumns { cursor } => {
                let cursor = *cursor;
                let last = self.column_order.len().saturating_sub(1);
                match code {
                    KeyCode::Left  | KeyCode::Char('h') => {
                        if let AppMode::ReorderColumns { cursor: c } = &mut self.mode { *c = c.saturating_sub(1); }
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if let AppMode::ReorderColumns { cursor: c } = &mut self.mode { *c = (*c + 1).min(last); }
                    }
                    // Shift+H / Shift+L move the column itself.
                    KeyCode::Char('H') => {
                        if cursor > 0 {
                            self.column_order.swap(cursor, cursor - 1);
                            if let AppMode::ReorderColumns { cursor: c } = &mut self.mode { *c -= 1; }
                        }
                    }
                    KeyCode::Char('L') => {
                        if cursor < last {
                            self.column_order.swap(cursor, cursor + 1);
                            if let AppMode::ReorderColumns { cursor: c } = &mut self.mode { *c += 1; }
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('o') => {
                        let _ = crate::settings::save_column_order(&self.column_order);
                        self.mode = AppMode::Normal;
                    }
                    _ => {}
                }
            }
            AppMode::Normal => self.handle_normal_key(code),
        }
    }

    fn handle_normal_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Down | KeyCode::Char('j') => self.next_row(),
            KeyCode::Up   | KeyCode::Char('k') => self.prev_row(),
            KeyCode::Enter => self.open_selected_url(),
            KeyCode::Char('c') => {
                let editor = ConfigEditor::new(&self.config, self.config_path.clone());
                self.mode = AppMode::Config(editor);
            }
            KeyCode::Char('o') => {
                self.mode = AppMode::ReorderColumns { cursor: 0 };
            }
            _ => {}
        }
    }

    fn handle_daemon_msg(&mut self, msg: DaemonMessage) {
        match msg {
            DaemonMessage::PollingStarted => {
                self.polling_until = Some(Instant::now() + Duration::from_secs(2));
            }
            DaemonMessage::PollingFinished => {}
            DaemonMessage::StateSnapshot { pull_requests } => {
                self.prs = pull_requests;
                self.status = ConnectionStatus::Connected;
                self.push_log(format!("snapshot: {} open PRs", self.prs.len()));
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
                self.push_log(format!("new  PR #{} {}  [{}]", pr.number, pr.title, pr.repo));
                self.prs.push(pr);
                if self.table_state.selected().is_none() {
                    self.table_state.select(Some(0));
                }
            }
            VcsEvent::PullRequestUpdated { old: _, new } => {
                self.push_log(format!("upd  PR #{} {}  [{}]", new.number, new.title, new.repo));
                if let Some(pos) = self.prs.iter().position(|p| p.number == new.number && p.repo == new.repo) {
                    self.prs[pos] = new;
                }
            }
            VcsEvent::PullRequestClosed(pr) => {
                self.push_log(format!("closed PR #{} {}  [{}]", pr.number, pr.title, pr.repo));
                self.prs.retain(|p| !(p.number == pr.number && p.repo == pr.repo));
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
        if self.prs.is_empty() { return; }
        let next = match self.table_state.selected() {
            Some(i) => (i + 1).min(self.prs.len() - 1),
            None    => 0,
        };
        self.table_state.select(Some(next));
    }

    fn prev_row(&mut self) {
        if self.prs.is_empty() { return; }
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

    pub fn push_log(&mut self, message: String) {
        if self.log.len() == MAX_LOG_ENTRIES { self.log.pop_front(); }
        self.log.push_back(LogEntry { timestamp: now_hms(), message });
    }
}

// ── Demo helpers ──────────────────────────────────────────────────────────────

fn pr(number: u64, repo: &str, title: &str, author: &str, state: &str, created_at: u64, draft: bool) -> PullRequest {
    PullRequest {
        id: number,
        number,
        title: title.to_string(),
        state: state.to_string(),
        url: String::new(),
        author: author.to_string(),
        repo: repo.to_string(),
        provider: "github".to_string(),
        created_at,
        draft,
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
