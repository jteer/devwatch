use std::time::Instant;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};

use crate::app::{App, ConnectionStatus};

const HEADER_FG: Color = Color::Cyan;
const SELECTED_FG: Color = Color::Yellow;
const DIM: Color = Color::DarkGray;
const NEW_COLOR: Color = Color::Green;
const UPD_COLOR: Color = Color::Yellow;
const CLO_COLOR: Color = Color::Red;

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
}

fn render_pr_table(frame: &mut Frame, app: &mut App, area: Rect) {
    let title = format!(" Pull Requests ({} open) ", app.prs.len());

    let header = Row::new(vec![
        Cell::from("  #").style(Style::new().fg(HEADER_FG).bold()),
        Cell::from("Repo").style(Style::new().fg(HEADER_FG).bold()),
        Cell::from("Title").style(Style::new().fg(HEADER_FG).bold()),
        Cell::from("Author").style(Style::new().fg(HEADER_FG).bold()),
        Cell::from("State").style(Style::new().fg(HEADER_FG).bold()),
    ])
    .height(1);

    let rows: Vec<Row> = app
        .prs
        .iter()
        .map(|pr| {
            Row::new(vec![
                Cell::from(format!("  #{}", pr.number)),
                Cell::from(pr.repo.clone()),
                Cell::from(pr.title.clone()),
                Cell::from(pr.author.clone()),
                Cell::from(pr.state.clone()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Length(24),
        Constraint::Min(30),
        Constraint::Length(16),
        Constraint::Length(8),
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

    let list = List::new(items).block(
        Block::new()
            .borders(Borders::ALL)
            .title(" Event Log "),
    );

    frame.render_widget(list, area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let keys = " ↑↓/jk navigate  Enter open URL  q quit";

    // Timer: seconds since the last real VCS event.
    let (timer_text, timer_style) = match app.event_timer() {
        None => ("no events yet".to_string(), Style::new().fg(DIM)),
        Some(s) if s < 60 => (format!("last event {s}s ago"), Style::new()),
        Some(s) => (format!("last event {}m {}s ago", s / 60, s % 60), Style::new()),
    };

    // Status: "Polling…" for 2s after each cycle starts, otherwise connection state.
    let is_polling = app.polling_until.map(|t| Instant::now() < t).unwrap_or(false);
    let (status_text, conn_style) = if is_polling {
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

    // Split right into timer and status spans at the bullet separator.
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
