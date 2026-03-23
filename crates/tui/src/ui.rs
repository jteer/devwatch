use std::time::{Instant, SystemTime, UNIX_EPOCH};

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table},
    Frame,
};

use crate::app::{App, AppMode, ConnectionStatus};
use crate::config_editor::{ConfigEditor, FocusedItem, RepoField};

const HEADER_FG:   Color = Color::Cyan;
const SELECTED_FG: Color = Color::Yellow;
const DIM:         Color = Color::DarkGray;
const NEW_COLOR:   Color = Color::Green;
const UPD_COLOR:   Color = Color::Yellow;
const CLO_COLOR:   Color = Color::Red;
const DRAFT_COLOR: Color = Color::DarkGray;

// ── Top-level draw ────────────────────────────────────────────────────────────

pub fn draw(frame: &mut Frame, app: &mut App) {
    let [table_area, log_area, status_area] = Layout::vertical([
        Constraint::Min(6),
        Constraint::Length(8),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    render_pr_table(frame, app, table_area);
    render_event_log(frame, app, log_area);
    render_status_bar(frame, app, status_area);

    // Overlay the config editor on top of everything when active.
    if let AppMode::Config(editor) = &mut app.mode {
        render_config_editor(frame, editor, frame.area());
    }
}

// ── PR table ──────────────────────────────────────────────────────────────────

fn render_pr_table(frame: &mut Frame, app: &mut App, area: Rect) {
    let title = if app.is_demo {
        format!(" Pull Requests ({} open)  [DEMO] ", app.prs.len())
    } else {
        format!(" Pull Requests ({} open) ", app.prs.len())
    };

    let header = Row::new(vec![
        Cell::from("  #").style(Style::new().fg(HEADER_FG).bold()),
        Cell::from("Repo").style(Style::new().fg(HEADER_FG).bold()),
        Cell::from("Title").style(Style::new().fg(HEADER_FG).bold()),
        Cell::from("Author").style(Style::new().fg(HEADER_FG).bold()),
        Cell::from("Age").style(Style::new().fg(HEADER_FG).bold()),
        Cell::from("State").style(Style::new().fg(HEADER_FG).bold()),
    ])
    .height(1);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let rows: Vec<Row> = app
        .prs
        .iter()
        .map(|pr| {
            let title_cell = if pr.draft {
                Cell::from(Line::from(vec![
                    Span::styled("[draft] ", Style::new().fg(DRAFT_COLOR)),
                    Span::raw(pr.title.clone()),
                ]))
            } else {
                Cell::from(pr.title.clone())
            };

            let age_cell = Cell::from(pr_age(now, pr.created_at)).style(Style::new().fg(DIM));

            let state_style = match pr.state.as_str() {
                "open"   => Style::new().fg(Color::Green),
                "merged" => Style::new().fg(Color::Magenta),
                "closed" => Style::new().fg(Color::Red),
                _        => Style::new().fg(DIM),
            };

            Row::new(vec![
                Cell::from(format!("  #{}", pr.number)),
                Cell::from(pr.repo.clone()),
                title_cell,
                Cell::from(pr.author.clone()),
                age_cell,
                Cell::from(pr.state.clone()).style(state_style),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(7),
        Constraint::Length(22),
        Constraint::Min(28),
        Constraint::Length(14),
        Constraint::Length(5),
        Constraint::Length(7),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::new().borders(Borders::ALL).title(title))
        .row_highlight_style(
            Style::default()
                .fg(SELECTED_FG)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::REVERSED),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn pr_age(now: u64, created_at: u64) -> String {
    if created_at == 0 { return "-".to_string(); }
    let secs = now.saturating_sub(created_at);
    if secs < 3600          { format!("{}m",  secs / 60) }
    else if secs < 86400    { format!("{}h",  secs / 3600) }
    else if secs < 86400*30 { format!("{}d",  secs / 86400) }
    else                    { format!("{}w",  secs / (86400*7)) }
}

// ── Event log ────────────────────────────────────────────────────────────────

fn render_event_log(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .log
        .iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .map(|entry| {
            let (prefix, color) = if entry.message.starts_with("new") {
                ("●", NEW_COLOR)
            } else if entry.message.starts_with("upd") {
                ("◆", UPD_COLOR)
            } else if entry.message.starts_with("closed") {
                ("○", CLO_COLOR)
            } else {
                ("·", DIM)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", entry.timestamp), Style::new().fg(DIM)),
                Span::styled(format!("{prefix} "), Style::new().fg(color).bold()),
                Span::raw(entry.message.clone()),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::new().borders(Borders::ALL).title(" Event Log "));
    frame.render_widget(list, area);
}

// ── Status bar ────────────────────────────────────────────────────────────────

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let keys = match &app.mode {
        AppMode::Config(_) => " ↑↓/jk navigate  Enter edit  Tab next field  a add  d delete  s save  Esc back",
        AppMode::Normal    => " ↑↓/jk navigate  Enter open URL  c config  q quit",
    };

    let (timer_text, timer_style) = match app.event_timer() {
        None          => ("no events yet".to_string(), Style::new().fg(DIM)),
        Some(s) if s < 60 => (format!("last event {s}s ago"), Style::new()),
        Some(s)       => (format!("last event {}m {}s ago", s / 60, s % 60), Style::new()),
    };

    let is_polling = app.polling_until.map(|t| Instant::now() < t).unwrap_or(false);
    let (status_text, conn_style) = if app.is_demo {
        ("Demo".to_string(), Style::new().fg(Color::Magenta))
    } else if is_polling {
        ("Polling…".to_string(), Style::new().fg(Color::Yellow))
    } else {
        let style = match app.status {
            ConnectionStatus::Connected    => Style::new().fg(Color::Green),
            ConnectionStatus::Connecting   => Style::new().fg(Color::Yellow),
            ConnectionStatus::Disconnected => Style::new().fg(Color::Red),
        };
        (app.status.to_string(), style)
    };

    let right = format!("{}  ●  {}  ", timer_text, status_text);
    let pad = (area.width as usize).saturating_sub(keys.len() + right.len());
    let bullet_pos = right.find('●').unwrap_or(right.len());
    let timer_part = &right[..bullet_pos];
    let conn_part  = &right[bullet_pos..];

    let bar = Paragraph::new(Line::from(vec![
        Span::styled(keys, Style::new().fg(DIM)),
        Span::raw(" ".repeat(pad)),
        Span::styled(timer_part, timer_style),
        Span::styled(conn_part, conn_style),
    ]));
    frame.render_widget(bar, area);
}

// ── Config editor overlay ─────────────────────────────────────────────────────

fn render_config_editor(frame: &mut Frame, editor: &mut ConfigEditor, area: Rect) {
    // Centre a dialog that's 70% wide and fills most of the height.
    let popup = centered_rect(72, 85, area);
    frame.render_widget(Clear, popup);

    let title = " Configuration ";
    let block = Block::new()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::new().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    // Split into: general fields, repo list, status bar.
    let [general_area, repo_area, status_area] = Layout::vertical([
        Constraint::Length(6),
        Constraint::Min(4),
        Constraint::Length(1),
    ])
    .areas(inner);

    render_cfg_general(frame, editor, general_area);
    render_cfg_repos(frame, editor, repo_area);
    render_cfg_status(frame, editor, status_area);
}

fn render_cfg_general(frame: &mut Frame, editor: &ConfigEditor, area: Rect) {
    let port_active     = editor.port_active();
    let interval_active = editor.interval_active();

    let port_style     = field_style(port_active);
    let interval_style = field_style(interval_active);

    let port_val     = field_value(&editor.port_buf,     port_active);
    let interval_val = field_value(&editor.interval_buf, interval_active);

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  daemon_port        ", Style::new().fg(HEADER_FG)),
            Span::styled(port_val, port_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  poll_interval_secs ", Style::new().fg(HEADER_FG)),
            Span::styled(interval_val, interval_style),
        ]),
        Line::from(""),
    ];

    frame.render_widget(Paragraph::new(text), area);
}

fn render_cfg_repos(frame: &mut Frame, editor: &mut ConfigEditor, area: Rect) {
    let in_repos = editor.focused == FocusedItem::Repos;
    let add_focused = editor.focused == FocusedItem::AddRepo;

    let mut items: Vec<ListItem> = editor
        .repos
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            let selected = in_repos
                && editor.repo_list_state.selected() == Some(i);

            // Render each sub-field with an edit cursor when active.
            let provider_span = {
                let active = selected && repo.editing == Some(RepoField::Provider);
                Span::styled(
                    format!("{:<8}", repo.provider),
                    if active { Style::new().fg(Color::Yellow).reversed() } else { Style::new() },
                )
            };
            let name_span = {
                let active = selected && repo.editing == Some(RepoField::Name);
                Span::styled(
                    field_value(&repo.name_buf, active),
                    if active { Style::new().fg(Color::Yellow) } else { Style::new() },
                )
            };
            let token_span = {
                let active = selected && repo.editing == Some(RepoField::Token);
                let display = if repo.token_buf.is_empty() && !active {
                    "(no token — uses env)".to_string()
                } else {
                    field_value(&repo.token_buf, active)
                };
                Span::styled(
                    display,
                    if active { Style::new().fg(Color::Yellow) }
                    else { Style::new().fg(DIM) },
                )
            };

            let line = Line::from(vec![
                Span::raw("  "),
                provider_span,
                Span::raw("  "),
                name_span,
                Span::raw("  "),
                token_span,
            ]);
            ListItem::new(line)
        })
        .collect();

    // "[+ Add Repo]" footer item.
    let add_style = if add_focused {
        Style::new().fg(Color::Green).bold()
    } else {
        Style::new().fg(DIM)
    };
    items.push(ListItem::new(Line::from(Span::styled("  [+ Add Repo]", add_style))));

    let repo_block = Block::new()
        .borders(Borders::TOP)
        .title(Span::styled(" Repositories ", Style::new().fg(HEADER_FG)));

    let list = List::new(items)
        .block(repo_block)
        .highlight_style(Style::new().fg(SELECTED_FG).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut editor.repo_list_state);
}

fn render_cfg_status(frame: &mut Frame, editor: &ConfigEditor, area: Rect) {
    let msg = editor.status_msg.as_deref().unwrap_or("");
    let style = if msg.starts_with("Error") || msg.starts_with("Write") || msg.starts_with("Serialise") {
        Style::new().fg(Color::Red)
    } else {
        Style::new().fg(Color::Green)
    };
    frame.render_widget(Paragraph::new(Span::styled(msg, style)), area);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns a cursor-appended string when the field is active.
fn field_value(buf: &str, active: bool) -> String {
    if active { format!("{buf}▋") } else { buf.to_string() }
}

fn field_style(active: bool) -> Style {
    if active { Style::new().fg(Color::Yellow) } else { Style::new() }
}

/// Compute a centred popup rect as a percentage of `r`.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
