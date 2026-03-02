use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let status = if app.events.paused {
        Span::styled(" PAUSED ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" LIVE ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    };

    let header = Row::new(vec![
        Cell::from("Timestamp"),
        Cell::from("Type"),
        Cell::from("Session"),
        Cell::from("Content URL"),
    ])
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .events
        .events
        .iter()
        .map(|e| {
            let url = e
                .content_url
                .as_deref()
                .unwrap_or("—");
            let url_display = if url.len() > 60 {
                format!("{}...", &url[..57])
            } else {
                url.to_string()
            };
            Row::new(vec![
                Cell::from(e.event_timestamp.format("%H:%M:%S").to_string()),
                Cell::from(e.event_type.clone()),
                Cell::from(e.session_id.to_string()[..8].to_string()),
                Cell::from(url_display),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Length(20),
        Constraint::Length(10),
        Constraint::Min(30),
    ];

    let title = format!(" Events ({}) ", app.events.events.len());
    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_bottom(status),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = TableState::default().with_selected(Some(app.events.selected));
    frame.render_stateful_widget(table, area, &mut state);
}
