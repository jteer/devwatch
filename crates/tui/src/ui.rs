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

use crate::app::{App, AppMode, ColumnId, ConnectionStatus, NotificationMode, SortDir, ToastKind};
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

    render_toasts(frame, app);
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
            } else if entry.message.starts_with("notif") {
                ("◉", t.event_notif)
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

    // ── Right side (always fully visible) ─────────────────────────────────────
    let (timer_text, timer_style) = match app.event_timer() {
        None              => ("no events".to_string(),                              Style::new().fg(t.dim)),
        Some(s) if s < 60 => (format!("last {s}s ago"),                            Style::new()),
        Some(s)           => (format!("last {}m{}s ago", s / 60, s % 60),          Style::new()),
    };
    let is_polling = app.polling_until.map(|u| Instant::now() < u).unwrap_or(false);
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
    let notif_icon = match app.notif_mode {
        NotificationMode::InApp => "🔔 in_app",
        NotificationMode::Os    => "🔔 os",
        NotificationMode::Both  => "🔔 both",
        NotificationMode::Off   => "🔕 off",
    };

    // Build the right-side line and measure its display width.
    let right_spans: Vec<Span> = vec![
        Span::styled(format!(" {notif_icon}  "),        Style::new().fg(t.dim)),
        Span::styled(format!("{timer_text}  ●  "),      timer_style),
        Span::styled(format!("{status_text} "),         conn_style),
    ];
    let right_width = right_spans.iter().map(|s| s.content.chars().count()).sum::<usize>()
        // each emoji renders as 2 columns; add 1 extra per emoji char
        + right_spans.iter().flat_map(|s| s.content.chars()).filter(|c| !c.is_ascii()).count()
        as usize;

    // ── Left side (key hints, truncated to remaining space) ────────────────────
    let hints_full = match &app.mode {
        AppMode::Config(_)             => " ↑↓/jk navigate  │  Enter edit  │  Tab next field  │  a add  │  d delete  │  s save  │  Esc back",
        AppMode::HeaderSelect { .. }   => " ←→/Tab col  │  Enter sort  │  ↑↓ rows  │  Esc cancel",
        AppMode::ReorderColumns { .. } => " ←→/hl select  │  H/L move  │  Esc done",
        AppMode::Filter                => " Type to filter  │  Enter/Esc close",
        AppMode::Normal if app.is_demo => " ↑↓ scroll  │  Enter open  │  / filter  │  s sort  │  o reorder  │  c config  │  m notif  │  n add event  │  q quit",
        AppMode::Normal                => " ↑↓ scroll  │  Enter open  │  / filter  │  s sort  │  o reorder  │  c config  │  m notif  │  q quit",
    };

    let available = (area.width as usize).saturating_sub(right_width);
    let hints = if hints_full.chars().count() <= available {
        hints_full.to_string()
    } else if available > 1 {
        // Convert column count → byte offset (all chars here are 1 column wide,
        // but some are multi-byte UTF-8, so we must use char_indices not a raw slice).
        let byte_end = hints_full
            .char_indices()
            .nth(available.saturating_sub(1))
            .map(|(i, _)| i)
            .unwrap_or(hints_full.len());
        let truncated = &hints_full[..byte_end];
        // Snap back to the last " │ " so we never cut a hint label in half.
        let boundary = truncated.rfind(" │ ").unwrap_or(truncated.len());
        format!("{}…", &truncated[..boundary])
    } else {
        String::new()
    };

    // ── Layout: hints left | right status right ────────────────────────────────
    let [left_area, right_area] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(right_width as u16),
    ])
    .areas(area);

    frame.render_widget(
        Paragraph::new(Span::styled(hints, Style::new().fg(t.dim))),
        left_area,
    );
    frame.render_widget(
        Paragraph::new(Line::from(right_spans)),
        right_area,
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

// ── Toast overlay ─────────────────────────────────────────────────────────────

fn render_toasts(frame: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = frame.area();
    const W: u16 = 50;
    const H: u16 = 3;

    // Count bottom rows occupied by fixed bars so toasts always float above them.
    let bottom_rows: u16 = 1  // status bar
        + if matches!(app.mode, AppMode::Filter) || !app.filter.is_empty() { 1 } else { 0 };

    for (i, toast) in app.toasts.iter().rev().take(3).enumerate() {
        let (label, color) = match toast.kind {
            ToastKind::New          => ("● New PR ", t.event_new),
            ToastKind::Updated      => ("◆ Updated", t.event_upd),
            ToastKind::Closed       => ("○ Closed ", t.event_clo),
            ToastKind::Notification => ("◉ Notif  ", t.event_notif),
        };

        let x = area.width.saturating_sub(W + 1);
        let y = area.height
            .saturating_sub(bottom_rows)
            .saturating_sub(H * (i as u16 + 1));

        let toast_area = Rect { x, y, width: W, height: H };
        frame.render_widget(Clear, toast_area);

        let max_msg = W.saturating_sub(14) as usize;
        let msg: String = toast.message.chars().take(max_msg).collect();

        let content = Line::from(vec![
            Span::styled(format!(" {label}  "), Style::new().fg(color).bold()),
            Span::raw(msg),
        ]);

        frame.render_widget(
            Paragraph::new(content)
                .block(Block::new().borders(Borders::ALL).border_style(Style::new().fg(color))),
            toast_area,
        );
    }
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
