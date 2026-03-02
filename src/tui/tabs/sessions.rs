use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    if app.sessions.showing_detail {
        render_detail(frame, area, app);
    } else {
        render_list(frame, area, app);
    }
}

fn render_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let header = Row::new(vec![
        Cell::from("ID"),
        Cell::from("Scope"),
        Cell::from("Outcome"),
        Cell::from("Started"),
        Cell::from("Ended"),
    ])
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .sessions
        .sessions
        .iter()
        .map(|s| {
            let id_short = &s.id.to_string()[..8];
            let scope = s.content_scope.as_deref().unwrap_or("—");
            let outcome = s.outcome_type.as_deref().unwrap_or("—");
            let started = s.started_at.format("%H:%M:%S").to_string();
            let ended = s
                .ended_at
                .map(|t| t.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| "active".to_string());
            Row::new(vec![
                Cell::from(id_short.to_string()),
                Cell::from(scope.to_string()),
                Cell::from(outcome.to_string()),
                Cell::from(started),
                Cell::from(ended),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Min(12),
        Constraint::Length(14),
        Constraint::Length(10),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default().borders(Borders::ALL).title(format!(
                " Sessions (page {}) ",
                app.sessions.page + 1
            )),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = TableState::default().with_selected(Some(app.sessions.selected));
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_detail(frame: &mut Frame, area: Rect, app: &App) {
    let Some(detail) = &app.sessions.detail else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Session Detail ");
        frame.render_widget(Paragraph::new("  Loading...").block(block), area);
        return;
    };

    let s = &detail.session;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(area);

    // Session info
    let ext_id = s
        .external_session_id
        .as_deref()
        .unwrap_or("—");
    let platform = s.platform_id.as_deref().unwrap_or("—");
    let agent = s.agent_id.as_deref().unwrap_or("—");
    let scope = s.content_scope.as_deref().unwrap_or("—");
    let outcome = s.outcome_type.as_deref().unwrap_or("—");

    let info_lines = vec![
        Line::from(format!(
            "  ID: {}   External: {ext_id}",
            s.id
        )),
        Line::from(format!(
            "  Platform: {platform}   Agent: {agent}   Scope: {scope}"
        )),
        Line::from(format!(
            "  Outcome: {outcome}   Started: {}   Ended: {}",
            s.started_at.format("%Y-%m-%d %H:%M:%S"),
            s.ended_at
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "active".to_string()),
        )),
    ];

    let info_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Session {} [Esc to go back] ", &s.id.to_string()[..8]));
    frame.render_widget(Paragraph::new(info_lines).block(info_block), chunks[0]);

    // Events table
    let header = Row::new(vec![
        Cell::from("#"),
        Cell::from("Type"),
        Cell::from("Content URL"),
        Cell::from("Timestamp"),
    ])
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = detail
        .events
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let url = e
                .content_url
                .as_deref()
                .unwrap_or("—");
            let url_display = if url.len() > 50 {
                format!("{}...", &url[..47])
            } else {
                url.to_string()
            };
            Row::new(vec![
                Cell::from(format!("{}", i + 1)),
                Cell::from(e.event_type.clone()),
                Cell::from(url_display),
                Cell::from(e.event_timestamp.format("%H:%M:%S").to_string()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(4),
        Constraint::Length(20),
        Constraint::Min(30),
        Constraint::Length(10),
    ];

    let events_table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Events ({}) ", detail.events.len())),
        );

    frame.render_widget(events_table, chunks[1]);
}
