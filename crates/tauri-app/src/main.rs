mod daemon;

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use devwatch_core::config::AppConfig;
use devwatch_core::types::PullRequest;

// ── State types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Disconnected,
}

pub struct AppState {
    pub prs:    Vec<PullRequest>,
    pub status: ConnectionStatus,
    pub unread: u32,
}

impl Default for AppState {
    fn default() -> Self {
        Self { prs: Vec::new(), status: ConnectionStatus::Connecting, unread: 0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub close_behaviour:   CloseBehaviour,
    pub notification_mode: NotificationMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseBehaviour {
    HideToTray,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationMode {
    InApp,
    Os,
    Both,
    Off,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            close_behaviour:   CloseBehaviour::HideToTray,
            notification_mode: NotificationMode::InApp,
        }
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
fn list_prs(state: State<'_, Mutex<AppState>>) -> Vec<PullRequest> {
    state.lock().unwrap().prs.clone()
}

#[tauri::command]
fn get_connection_status(state: State<'_, Mutex<AppState>>) -> String {
    match state.lock().unwrap().status {
        ConnectionStatus::Connected    => "connected".into(),
        ConnectionStatus::Connecting   => "connecting".into(),
        ConnectionStatus::Disconnected => "disconnected".into(),
    }
}

#[tauri::command]
fn get_unread_count(state: State<'_, Mutex<AppState>>) -> u32 {
    state.lock().unwrap().unread
}

#[tauri::command]
fn mark_all_read(state: State<'_, Mutex<AppState>>, app: AppHandle) {
    state.lock().unwrap().unread = 0;
    let _ = app.emit("unread-count", 0u32);
}

#[tauri::command]
fn open_pr(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_config() -> Result<AppConfig, String> {
    let raw = std::fs::read_to_string(find_config_path()).unwrap_or_default();
    toml::from_str(&raw).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    let s = toml::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(find_config_path(), s).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_app_settings(state: State<'_, Mutex<AppSettings>>) -> AppSettings {
    state.lock().unwrap().clone()
}

#[tauri::command]
fn save_app_settings(settings: AppSettings, state: State<'_, Mutex<AppSettings>>) {
    *state.lock().unwrap() = settings;
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .manage(Mutex::new(AppState::default()))
        .manage(Mutex::new(AppSettings::default()))
        .invoke_handler(tauri::generate_handler![
            list_prs,
            get_connection_status,
            get_unread_count,
            mark_all_read,
            open_pr,
            read_config,
            save_config,
            get_app_settings,
            save_app_settings,
        ])
        .setup(|app| {
            let quit   = MenuItem::with_id(app, "quit",   "Quit devwatch",  true, None::<&str>)?;
            let show   = MenuItem::with_id(app, "show",   "Open devwatch",  true, None::<&str>)?;
            let mark   = MenuItem::with_id(app, "mark",   "Mark all read",  true, None::<&str>)?;
            let menu   = Menu::with_items(app, &[&show, &mark, &quit])?;

            TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("devwatch")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => app.exit(0),
                    "show" => show_main_window(app),
                    "mark" => {
                        app.state::<Mutex<AppState>>().lock().unwrap().unread = 0;
                        let _ = app.emit("unread-count", 0u32);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // Start daemon connection background task.
            let port   = load_port();
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                daemon::run(handle, port).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let settings = window.app_handle().state::<Mutex<AppSettings>>();
                if settings.lock().unwrap().close_behaviour == CloseBehaviour::HideToTray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error running devwatch");
}

fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn load_port() -> u16 {
    std::fs::read_to_string(find_config_path())
        .ok()
        .and_then(|s| toml::from_str::<AppConfig>(&s).ok())
        .map(|c| c.daemon_port)
        .unwrap_or(7878)
}

/// Walk up from the current directory until we find `config.toml`, falling
/// back to `./config.toml` if nothing is found.
pub fn find_config_path() -> std::path::PathBuf {
    if let Ok(mut dir) = std::env::current_dir() {
        loop {
            let candidate = dir.join("config.toml");
            if candidate.exists() {
                return candidate;
            }
            if !dir.pop() {
                break;
            }
        }
    }
    std::path::PathBuf::from("config.toml")
}
