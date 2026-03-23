//! In-TUI configuration editor.
//!
//! Presents the current `config.toml` as an editable form.  Changes are only
//! written to disk when the user presses `s`; pressing `Esc` discards them.

use std::path::PathBuf;

use crossterm::event::KeyCode;
use ratatui::widgets::ListState;

use devwatch_core::config::{AppConfig, RepoConfig};

// ── Public types ─────────────────────────────────────────────────────────────

/// Top-level state for the config editor screen.
pub struct ConfigEditor {
    /// Editable copy of the port (kept as a string for inline editing).
    pub port_buf: String,
    pub interval_buf: String,
    pub repos: Vec<RepoRow>,
    /// Which "row" in the overall list is highlighted.
    pub focused: FocusedItem,
    /// Scroll state for the repo sub-list.
    pub repo_list_state: ListState,
    /// Path to write on save.
    pub config_path: PathBuf,
    /// Feedback message shown at the bottom (e.g. "Saved!" or an error).
    pub status_msg: Option<String>,
}

#[derive(Clone, PartialEq)]
pub enum FocusedItem {
    Port,
    Interval,
    Repos,   // focus is inside the repo list
    AddRepo,
}

/// A single repo entry inside the editor, with optional inline sub-editing.
pub struct RepoRow {
    pub provider: String,
    pub name_buf: String,
    pub token_buf: String,
    /// Which sub-field (if any) is currently being edited.
    pub editing: Option<RepoField>,
}

#[derive(Clone, PartialEq)]
pub enum RepoField {
    Provider,
    Name,
    Token,
}

/// Return value from `handle_key` so the caller knows what happened.
pub enum ConfigAction {
    None,
    Exit,
}

// ── Impl ─────────────────────────────────────────────────────────────────────

impl ConfigEditor {
    /// Build an editor pre-populated from `cfg`.  Writes back to `config_path`.
    pub fn new(cfg: &AppConfig, config_path: PathBuf) -> Self {
        let repos = cfg
            .repos
            .iter()
            .map(|r| RepoRow {
                provider:  r.provider.clone(),
                name_buf:  r.name.clone(),
                token_buf: r.token.clone().unwrap_or_default(),
                editing:   None,
            })
            .collect();

        Self {
            port_buf:      cfg.daemon_port.to_string(),
            interval_buf:  cfg.poll_interval_secs.to_string(),
            repos,
            focused:        FocusedItem::Port,
            repo_list_state: ListState::default().with_selected(Some(0)),
            config_path,
            status_msg:    None,
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) -> ConfigAction {
        // ── Repo sub-editing ─────────────────────────────────────────────────
        if self.focused == FocusedItem::Repos {
            if let Some(idx) = self.repo_list_state.selected() {
                if idx < self.repos.len() {
                    if self.repos[idx].editing.is_some() {
                        return self.handle_repo_edit_key(idx, code);
                    }
                }
            }
        }

        // ── Navigation / commands ────────────────────────────────────────────
        match code {
            KeyCode::Char('j') | KeyCode::Down  => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up    => self.move_up(),
            KeyCode::Enter                       => self.activate(),
            KeyCode::Backspace                   => self.backspace_active_field(),
            KeyCode::Char('a')                   => self.add_repo(),
            KeyCode::Char('d')                   => self.delete_repo(),
            KeyCode::Char('s')                   => self.save(),
            KeyCode::Char(c)                     => self.append_to_active_field(c),
            KeyCode::Esc                         => return ConfigAction::Exit,
            _                                    => {}
        }
        ConfigAction::None
    }

    // ── Repo sub-field editing ────────────────────────────────────────────────

    fn handle_repo_edit_key(&mut self, idx: usize, code: KeyCode) -> ConfigAction {
        let repo = &mut self.repos[idx];
        match repo.editing.clone().unwrap() {
            RepoField::Provider => match code {
                KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') => {
                    repo.provider = if repo.provider == "github" {
                        "gitlab".to_string()
                    } else {
                        "github".to_string()
                    };
                }
                KeyCode::Tab | KeyCode::Enter => repo.editing = Some(RepoField::Name),
                KeyCode::Esc => { repo.editing = None; }
                _ => {}
            },
            RepoField::Name => match code {
                KeyCode::Char(c)   => repo.name_buf.push(c),
                KeyCode::Backspace => { repo.name_buf.pop(); }
                KeyCode::Tab | KeyCode::Enter => repo.editing = Some(RepoField::Token),
                KeyCode::Esc => { repo.editing = None; }
                _ => {}
            },
            RepoField::Token => match code {
                KeyCode::Char(c)   => repo.token_buf.push(c),
                KeyCode::Backspace => { repo.token_buf.pop(); }
                KeyCode::Tab | KeyCode::Enter => repo.editing = None, // done
                KeyCode::Esc => { repo.editing = None; }
                _ => {}
            },
        }
        ConfigAction::None
    }

    // ── Field editing (port / interval) ──────────────────────────────────────

    fn backspace_active_field(&mut self) {
        match &self.focused {
            FocusedItem::Port     => { self.port_buf.pop(); }
            FocusedItem::Interval => { self.interval_buf.pop(); }
            _ => {}
        }
    }

    fn append_to_active_field(&mut self, c: char) {
        if !c.is_ascii_digit() { return; } // port & interval are numeric only
        match &self.focused {
            FocusedItem::Port     => self.port_buf.push(c),
            FocusedItem::Interval => self.interval_buf.push(c),
            _ => {}
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    fn move_up(&mut self) {
        match &self.focused {
            FocusedItem::Port     => {}
            FocusedItem::Interval => self.focused = FocusedItem::Port,
            FocusedItem::Repos    => {
                let sel = self.repo_list_state.selected().unwrap_or(0);
                if sel == 0 {
                    self.focused = FocusedItem::Interval;
                } else {
                    self.repo_list_state.select(Some(sel - 1));
                }
            }
            FocusedItem::AddRepo  => {
                if self.repos.is_empty() {
                    self.focused = FocusedItem::Interval;
                } else {
                    self.focused = FocusedItem::Repos;
                    self.repo_list_state.select(Some(self.repos.len() - 1));
                }
            }
        }
    }

    fn move_down(&mut self) {
        match &self.focused {
            FocusedItem::Port => self.focused = FocusedItem::Interval,
            FocusedItem::Interval => {
                if self.repos.is_empty() {
                    self.focused = FocusedItem::AddRepo;
                } else {
                    self.focused = FocusedItem::Repos;
                    self.repo_list_state.select(Some(0));
                }
            }
            FocusedItem::Repos => {
                let sel = self.repo_list_state.selected().unwrap_or(0);
                if sel + 1 < self.repos.len() {
                    self.repo_list_state.select(Some(sel + 1));
                } else {
                    self.focused = FocusedItem::AddRepo;
                }
            }
            FocusedItem::AddRepo => {}
        }
    }

    // ── Actions ───────────────────────────────────────────────────────────────

    fn activate(&mut self) {
        match &self.focused {
            FocusedItem::Port | FocusedItem::Interval => {
                // Already editable — nothing extra to do.
            }
            FocusedItem::Repos => {
                if let Some(idx) = self.repo_list_state.selected() {
                    if idx < self.repos.len() {
                        self.repos[idx].editing = Some(RepoField::Provider);
                    }
                }
            }
            FocusedItem::AddRepo => self.add_repo(),
        }
    }

    fn add_repo(&mut self) {
        self.repos.push(RepoRow {
            provider:  "github".to_string(),
            name_buf:  String::new(),
            token_buf: String::new(),
            editing:   Some(RepoField::Provider),
        });
        self.focused = FocusedItem::Repos;
        self.repo_list_state.select(Some(self.repos.len() - 1));
    }

    fn delete_repo(&mut self) {
        if self.focused != FocusedItem::Repos { return; }
        if let Some(idx) = self.repo_list_state.selected() {
            if idx < self.repos.len() {
                self.repos.remove(idx);
                let new_sel = if self.repos.is_empty() {
                    self.focused = FocusedItem::AddRepo;
                    None
                } else {
                    Some(idx.min(self.repos.len() - 1))
                };
                self.repo_list_state.select(new_sel);
            }
        }
    }

    fn save(&mut self) {
        let port: u16 = match self.port_buf.parse() {
            Ok(p) => p,
            Err(_) => { self.status_msg = Some("Error: daemon_port must be a valid port number".into()); return; }
        };
        let interval: u64 = match self.interval_buf.parse() {
            Ok(i) => i,
            Err(_) => { self.status_msg = Some("Error: poll_interval_secs must be a number".into()); return; }
        };

        let repos: Vec<RepoConfig> = self
            .repos
            .iter()
            .filter(|r| !r.name_buf.trim().is_empty())
            .map(|r| RepoConfig {
                provider: r.provider.clone(),
                name:     r.name_buf.trim().to_string(),
                token:    if r.token_buf.trim().is_empty() { None } else { Some(r.token_buf.trim().to_string()) },
            })
            .collect();

        let cfg = AppConfig { daemon_port: port, poll_interval_secs: interval, repos };

        match toml::to_string_pretty(&cfg) {
            Err(e) => { self.status_msg = Some(format!("Serialise error: {e}")); }
            Ok(toml_str) => match std::fs::write(&self.config_path, toml_str) {
                Ok(_)  => { self.status_msg = Some(format!("Saved to {}", self.config_path.display())); }
                Err(e) => { self.status_msg = Some(format!("Write error: {e}")); }
            },
        }
    }

    // ── Helpers for rendering ─────────────────────────────────────────────────

    /// True if the port field is currently the active text target.
    pub fn port_active(&self) -> bool   { self.focused == FocusedItem::Port }
    /// True if the interval field is currently the active text target.
    pub fn interval_active(&self) -> bool { self.focused == FocusedItem::Interval }
}
