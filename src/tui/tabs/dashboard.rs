use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(7), // health + session counts row
            Constraint::Length(5), // event rate
            Constraint::Min(0),   // spacer
        ])
        .split(area);

    let top_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[0]);

    render_health(frame, top_row[0], app);
    render_session_counts(frame, top_row[1], app);
    render_event_rate(frame, chunks[1], app);
}

fn render_health(frame: &mut Frame, area: Rect, app: &App) {
    let d = &app.dashboard;

    let health_icon = match d.healthy {
        Some(true) => Span::styled("● Connected", Style::default().fg(Color::Green)),
        Some(false) => Span::styled("● Disconnected", Style::default().fg(Color::Red)),
        None => Span::styled("○ Checking...", Style::default().fg(Color::DarkGray)),
    };

    let latency = d
        .db_latency_ms
        .map(|ms| format!("{ms}ms"))
        .unwrap_or_else(|| "—".into());

    let domains = d
        .domain_count
        .map(|n| format!("{n}"))
        .unwrap_or_else(|| "—".into());

    let lines = vec![
        Line::from(vec![Span::raw("  Status: "), health_icon]),
        Line::from(format!("  Ping:   {latency}")),
        Line::from(format!("  Domains: {domains} registered")),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Server Health ");
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_session_counts(frame: &mut Frame, area: Rect, app: &App) {
    let lines = if let Some(c) = &app.dashboard.session_counts {
        let pct = |n: i64| -> String {
            if c.total == 0 {
                "0%".to_string()
            } else {
                format!("{}%", n * 100 / c.total)
            }
        };

        vec![
            Line::from(vec![
                Span::raw("  Total: "),
                Span::styled(
                    format!("{:>8}", c.total),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(format!(
                "  Active: {:>7}   Ended: {}",
                c.active, c.ended
            )),
            Line::from(""),
            Line::from(format!(
                "  Conversions: {:>5} ({})   Abandonments: {} ({})   Browse: {} ({})",
                c.conversions,
                pct(c.conversions),
                c.abandonments,
                pct(c.abandonments),
                c.browses,
                pct(c.browses),
            )),
        ]
    } else {
        vec![Line::from("  Loading...")]
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Session Counts ");
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_event_rate(frame: &mut Frame, area: Rect, app: &App) {
    let rate = app
        .dashboard
        .events_per_minute
        .map(|n| format!("{n}"))
        .unwrap_or_else(|| "—".into());

    let lines = vec![Line::from(format!("  Events/min: {rate}"))];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Event Rate (last 1 min) ");
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}
