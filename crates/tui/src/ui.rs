use std::time::{Instant, SystemTime, UNIX_EPOCH};

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row,
        Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
    },
    Frame,
};

use crate::app::{App, AppMode, ColumnId, ConnectionStatus};
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

    if let AppMode::Config(editor) = &mut app.mode {
        render_config_editor(frame, editor, frame.area());
    }
}

// ── PR table ──────────────────────────────────────────────────────────────────

fn render_pr_table(frame: &mut Frame, app: &mut App, area: Rect) {
    let reorder_cursor = match &app.mode {
        AppMode::ReorderColumns { cursor } => Some(*cursor),
        _ => None,
    };

    let title = if app.is_demo {
        format!(" Pull Requests ({} open)  [DEMO] ", app.prs.len())
    } else {
        format!(" Pull Requests ({} open) ", app.prs.len())
    };

    // Build header cells — highlight the selected column in reorder mode.
    let header_cells: Vec<Cell> = app
        .column_order
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let style = if reorder_cursor == Some(i) {
                // Inverted cyan so the column stands out.
                Style::new().fg(Color::Black).bg(HEADER_FG).bold()
            } else {
                Style::new().fg(HEADER_FG).bold()
            };
            Cell::from(col.header()).style(style)
        })
        .collect();
    let header = Row::new(header_cells).height(1);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let rows: Vec<Row> = app
        .prs
        .iter()
        .map(|pr| {
            // Pre-build each possible cell.
            let number_cell = Cell::from(format!("  #{}", pr.number));
            let repo_cell   = Cell::from(pr.repo.clone());
            let title_cell  = if pr.draft {
                Cell::from(Line::from(vec![
                    Span::styled("[draft] ", Style::new().fg(DRAFT_COLOR)),
                    Span::raw(pr.title.clone()),
                ]))
            } else {
                Cell::from(pr.title.clone())
            };
            let author_cell = Cell::from(pr.author.clone());
            let age_cell    = Cell::from(pr_age(now, pr.created_at)).style(Style::new().fg(DIM));
            let state_style = match pr.state.as_str() {
                "open"   => Style::new().fg(Color::Green),
                "merged" => Style::new().fg(Color::Magenta),
                "closed" => Style::new().fg(Color::Red),
                _        => Style::new().fg(DIM),
            };
            let state_cell = Cell::from(pr.state.clone()).style(state_style);

            // Emit cells in the user-defined column order.
            let cells: Vec<Cell> = app
                .column_order
                .iter()
                .map(|col| match col {
                    ColumnId::Number => number_cell.clone(),
                    ColumnId::Repo   => repo_cell.clone(),
                    ColumnId::Title  => title_cell.clone(),
                    ColumnId::Author => author_cell.clone(),
                    ColumnId::Age    => age_cell.clone(),
                    ColumnId::State  => state_cell.clone(),
                })
                .collect();
            Row::new(cells)
        })
        .collect();

    let widths: Vec<Constraint> = app
        .column_order
        .iter()
        .map(|col| match col {
            ColumnId::Number => Constraint::Length(7),
            ColumnId::Repo   => Constraint::Length(22),
            ColumnId::Title  => Constraint::Min(28),
            ColumnId::Author => Constraint::Length(14),
            ColumnId::Age    => Constraint::Length(5),
            ColumnId::State  => Constraint::Length(7),
        })
        .collect();

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

    // Scrollbar — rendered on the right inner edge of the table block.
    // Only draw when there's something to scroll.
    let visible_rows = area.height.saturating_sub(3) as usize; // borders + header
    if app.prs.len() > visible_rows {
        let scroll_area = Rect {
            x:      area.x + area.width - 2,
            y:      area.y + 2,
            width:  1,
            height: area.height.saturating_sub(3),
        };
        let selected = app.table_state.selected().unwrap_or(0);
        let mut sb_state = ScrollbarState::new(app.prs.len())
            .viewport_content_length(visible_rows)
            .position(selected);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            scroll_area,
            &mut sb_state,
        );
    }
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

    frame.render_widget(
        List::new(items).block(Block::new().borders(Borders::ALL).title(" Event Log ")),
        area,
    );
}

// ── Status bar ────────────────────────────────────────────────────────────────

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let keys = match &app.mode {
        AppMode::Config(_)            => " ↑↓/jk navigate  │  Enter edit  │  Tab next field  │  a add  │  d delete  │  s save  │  Esc back",
        AppMode::ReorderColumns { .. } => " ←→/hl select column  │  H/L move column  │  Esc done",
        AppMode::Normal               => " ↑↓/jk navigate  │  Enter open URL  │  o reorder cols  │  c config  │  q quit",
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

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(keys, Style::new().fg(DIM)),
            Span::raw(" ".repeat(pad)),
            Span::styled(&right[..bullet_pos], timer_style),
            Span::styled(&right[bullet_pos..], conn_style),
        ])),
        area,
    );
}

// ── Config editor overlay ─────────────────────────────────────────────────────

fn render_config_editor(frame: &mut Frame, editor: &mut ConfigEditor, area: Rect) {
    let popup = centered_rect(72, 85, area);
    frame.render_widget(Clear, popup);

    let block = Block::new()
        .borders(Borders::ALL)
        .title(" Configuration ")
        .style(Style::new().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

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

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  daemon_port        ", Style::new().fg(HEADER_FG)),
                Span::styled(field_value(&editor.port_buf, port_active), field_style(port_active)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  poll_interval_secs ", Style::new().fg(HEADER_FG)),
                Span::styled(field_value(&editor.interval_buf, interval_active), field_style(interval_active)),
            ]),
            Line::from(""),
        ]),
        area,
    );
}

fn render_cfg_repos(frame: &mut Frame, editor: &mut ConfigEditor, area: Rect) {
    let in_repos    = editor.focused == FocusedItem::Repos;
    let add_focused = editor.focused == FocusedItem::AddRepo;

    let mut items: Vec<ListItem> = editor
        .repos
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            let selected = in_repos && editor.repo_list_state.selected() == Some(i);

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
                let display = if active {
                    field_value(&repo.token_buf, true)
                } else if repo.token_buf.is_empty() {
                    "(no token — uses env)".to_string()
                } else {
                    "••••••••••••••••".to_string()
                };
                Span::styled(
                    display,
                    if active { Style::new().fg(Color::Yellow) } else { Style::new().fg(DIM) },
                )
            };

            ListItem::new(Line::from(vec![
                Span::raw("  "),
                provider_span,
                Span::raw("  "),
                name_span,
                Span::raw("  "),
                token_span,
            ]))
        })
        .collect();

    let add_style = if add_focused { Style::new().fg(Color::Green).bold() } else { Style::new().fg(DIM) };
    items.push(ListItem::new(Line::from(Span::styled("  [+ Add Repo]", add_style))));

    let list = List::new(items)
        .block(Block::new().borders(Borders::TOP).title(Span::styled(" Repositories ", Style::new().fg(HEADER_FG))))
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

fn field_value(buf: &str, active: bool) -> String {
    if active { format!("{buf}▋") } else { buf.to_string() }
}

fn field_style(active: bool) -> Style {
    if active { Style::new().fg(Color::Yellow) } else { Style::new() }
}

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
