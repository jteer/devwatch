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

use crate::app::{App, AppMode, ColumnId, ConnectionStatus, SortDir};
use crate::config_editor::{ConfigEditor, FocusedItem, RepoField};
use crate::theme::Theme;

// ── Top-level draw ────────────────────────────────────────────────────────────

pub fn draw(frame: &mut Frame, app: &mut App) {
    // Live theme preview while config editor is open.
    if let AppMode::Config(editor) = &app.mode {
        app.theme = Theme::from_name(&editor.theme_buf);
    }

    let filter_active = matches!(app.mode, AppMode::Filter) || !app.filter.is_empty();
    let [table_area, log_area, filter_area, status_area] = Layout::vertical([
        Constraint::Min(6),
        Constraint::Length(8),
        Constraint::Length(if filter_active { 1 } else { 0 }),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    render_pr_table(frame, app, table_area);
    render_event_log(frame, app, log_area);
    if filter_active {
        render_filter_bar(frame, app, filter_area);
    }
    render_status_bar(frame, app, status_area);

    if let AppMode::Config(editor) = &mut app.mode {
        render_config_editor(frame, editor, &app.theme, frame.area());
    }
}

// ── PR table ──────────────────────────────────────────────────────────────────

fn render_pr_table(frame: &mut Frame, app: &mut App, area: Rect) {
    let t = &app.theme;
    let reorder_cursor = match &app.mode {
        AppMode::ReorderColumns { cursor } => Some(*cursor),
        _ => None,
    };
    let header_select_cursor = match &app.mode {
        AppMode::HeaderSelect { cursor } => Some(*cursor),
        _ => None,
    };

    let visible = app.visible_prs();
    let pr_count = visible.len();

    let title = if app.is_demo {
        format!(" Pull Requests ({} open)  [DEMO] ", pr_count)
    } else if !app.filter.is_empty() {
        format!(" Pull Requests ({} matching) ", pr_count)
    } else {
        format!(" Pull Requests ({} open) ", pr_count)
    };

    // Build header cells — indicators for reorder/sort cursor and sort direction.
    let header_cells: Vec<Cell> = app
        .column_order
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let sort_indicator = app.sort.as_ref().and_then(|(sc, dir)| {
                if sc == col { Some(if *dir == SortDir::Asc { " ▲" } else { " ▼" }) } else { None }
            }).unwrap_or("");

            let label = format!("{}{}", col.header(), sort_indicator);

            let style = if reorder_cursor == Some(i) {
                Style::new().fg(Color::Black).bg(t.header).bold()
            } else if header_select_cursor == Some(i) {
                Style::new()
                    .fg(Color::Black)
                    .bg(t.header)
                    .bold()
                    .add_modifier(Modifier::UNDERLINED)
            } else {
                Style::new().fg(t.header).bold()
            };
            Cell::from(label).style(style)
        })
        .collect();
    let header = Row::new(header_cells).height(1);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let rows: Vec<Row> = visible
        .iter()
        .map(|pr| {
            let number_cell = Cell::from(format!("  #{}", pr.number));
            let repo_cell   = Cell::from(pr.repo.clone());
            let title_cell  = if pr.draft {
                Cell::from(Line::from(vec![
                    Span::styled("[draft] ", Style::new().fg(t.draft)),
                    Span::raw(pr.title.clone()),
                ]))
            } else {
                Cell::from(pr.title.clone())
            };
            let author_cell = Cell::from(pr.author.clone());
            let age_cell    = Cell::from(pr_age(now, pr.created_at)).style(Style::new().fg(t.dim));
            let state_style = match pr.state.as_str() {
                "open"   => Style::new().fg(t.state_open),
                "merged" => Style::new().fg(t.state_merged),
                "closed" => Style::new().fg(t.state_closed),
                _        => Style::new().fg(t.dim),
            };
            let state_cell = Cell::from(pr.state.clone()).style(state_style);

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
                .fg(t.selected)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::REVERSED),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, area, &mut app.table_state);

    // Scrollbar on right inner edge.
    let visible_rows = area.height.saturating_sub(3) as usize;
    if pr_count > visible_rows {
        let scroll_area = Rect {
            x:      area.x + area.width - 2,
            y:      area.y + 2,
            width:  1,
            height: area.height.saturating_sub(3),
        };
        let selected = app.table_state.selected().unwrap_or(0);
        let mut sb_state = ScrollbarState::new(pr_count)
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
    let t = &app.theme;
    let items: Vec<ListItem> = app
        .log
        .iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .map(|entry| {
            let (prefix, color) = if entry.message.starts_with("new") {
                ("●", t.event_new)
            } else if entry.message.starts_with("upd") {
                ("◆", t.event_upd)
            } else if entry.message.starts_with("closed") {
                ("○", t.event_clo)
            } else {
                ("·", t.dim)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", entry.timestamp), Style::new().fg(t.dim)),
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

// ── Filter bar ────────────────────────────────────────────────────────────────

fn render_filter_bar(frame: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let editing = matches!(app.mode, AppMode::Filter);
    let cursor  = if editing { "▋" } else { "" };
    let label   = Span::styled(" Filter: ", Style::new().fg(t.header));
    let text    = Span::styled(
        format!("{}{}", app.filter, cursor),
        Style::new().fg(if editing { t.selected } else { Color::Reset }),
    );
    let hint = if !editing && !app.filter.is_empty() {
        Span::styled("  (Esc to clear)", Style::new().fg(t.dim))
    } else {
        Span::raw("")
    };
    frame.render_widget(Paragraph::new(Line::from(vec![label, text, hint])), area);
}

// ── Status bar ────────────────────────────────────────────────────────────────

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let keys = match &app.mode {
        AppMode::Config(_)             => " ↑↓/jk navigate  │  Enter edit  │  Tab next field  │  a add  │  d delete  │  s save  │  Esc back",
        AppMode::HeaderSelect { .. }   => " [←→/Tab] select column  │  Enter sort  │  [↑↓] back to rows  │  Esc cancel",
        AppMode::ReorderColumns { .. } => " ←→/hl select column  │  H/L move column  │  Esc done",
        AppMode::Filter                => " Type to filter  │  Enter/Esc close bar",
        AppMode::Normal                => " [↑↓] scroll  [←→/Tab] select column  │  Enter open  │  / filter  │  s sort  │  o reorder  │  c config  │  q quit",
    };

    let (timer_text, timer_style) = match app.event_timer() {
        None          => ("no events yet".to_string(), Style::new().fg(t.dim)),
        Some(s) if s < 60 => (format!("last event {s}s ago"), Style::new()),
        Some(s)       => (format!("last event {}m {}s ago", s / 60, s % 60), Style::new()),
    };

    let is_polling = app.polling_until.map(|t| Instant::now() < t).unwrap_or(false);
    let (status_text, conn_style) = if app.is_demo {
        ("Demo".to_string(), Style::new().fg(t.status_demo))
    } else if is_polling {
        ("Polling…".to_string(), Style::new().fg(t.status_warn))
    } else {
        let style = match app.status {
            ConnectionStatus::Connected    => Style::new().fg(t.status_ok),
            ConnectionStatus::Connecting   => Style::new().fg(t.status_warn),
            ConnectionStatus::Disconnected => Style::new().fg(t.status_err),
        };
        (app.status.to_string(), style)
    };

    let right = format!("{}  ●  {}  ", timer_text, status_text);
    let pad = (area.width as usize).saturating_sub(keys.len() + right.len());
    let bullet_pos = right.find('●').unwrap_or(right.len());

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(keys, Style::new().fg(t.dim)),
            Span::raw(" ".repeat(pad)),
            Span::styled(&right[..bullet_pos], timer_style),
            Span::styled(&right[bullet_pos..], conn_style),
        ])),
        area,
    );
}

// ── Config editor overlay ─────────────────────────────────────────────────────

fn render_config_editor(frame: &mut Frame, editor: &mut ConfigEditor, theme: &Theme, area: Rect) {
    let popup = centered_rect(72, 85, area);
    frame.render_widget(Clear, popup);

    let block = Block::new()
        .borders(Borders::ALL)
        .title(" Configuration ")
        .style(Style::new().fg(theme.header));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let [general_area, repo_area, status_area] = Layout::vertical([
        Constraint::Length(8),
        Constraint::Min(4),
        Constraint::Length(1),
    ])
    .areas(inner);

    render_cfg_general(frame, editor, theme, general_area);
    render_cfg_repos(frame, editor, theme, repo_area);
    render_cfg_status(frame, editor, theme, status_area);
}

fn render_cfg_general(frame: &mut Frame, editor: &ConfigEditor, theme: &Theme, area: Rect) {
    let port_active     = editor.port_active();
    let interval_active = editor.interval_active();
    let theme_active    = editor.theme_active();

    let theme_display = format!("{}  (Space/←/→ to cycle)", editor.theme_buf);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  daemon_port        ", Style::new().fg(theme.header)),
                Span::styled(field_value(&editor.port_buf, port_active), field_style(port_active)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  poll_interval_secs ", Style::new().fg(theme.header)),
                Span::styled(field_value(&editor.interval_buf, interval_active), field_style(interval_active)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  theme              ", Style::new().fg(theme.header)),
                Span::styled(
                    if theme_active { theme_display } else { editor.theme_buf.clone() },
                    if theme_active { Style::new().fg(Color::Yellow) } else { Style::new() },
                ),
            ]),
            Line::from(""),
        ]),
        area,
    );
}

fn render_cfg_repos(frame: &mut Frame, editor: &mut ConfigEditor, theme: &Theme, area: Rect) {
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
                    if active { Style::new().fg(Color::Yellow) } else { Style::new().fg(theme.dim) },
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

    let add_style = if add_focused { Style::new().fg(theme.state_open).bold() } else { Style::new().fg(theme.dim) };
    items.push(ListItem::new(Line::from(Span::styled("  [+ Add Repo]", add_style))));

    let list = List::new(items)
        .block(Block::new().borders(Borders::TOP).title(Span::styled(" Repositories ", Style::new().fg(theme.header))))
        .highlight_style(Style::new().fg(theme.selected).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut editor.repo_list_state);
}

fn render_cfg_status(frame: &mut Frame, editor: &ConfigEditor, theme: &Theme, area: Rect) {
    let msg = editor.status_msg.as_deref().unwrap_or("");
    let style = if msg.starts_with("Error") || msg.starts_with("Write") || msg.starts_with("Serialise") {
        Style::new().fg(theme.status_err)
    } else {
        Style::new().fg(theme.status_ok)
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
