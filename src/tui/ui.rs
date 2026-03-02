use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

use crate::app::App;
use crate::tabs::Tab;
use crate::tabs::{dashboard, events, platforms, publishers, resolve, sessions};

pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tab bar
            Constraint::Min(0),   // content
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    render_tab_bar(frame, chunks[0], app.active_tab);

    match app.active_tab {
        Tab::Dashboard => dashboard::render(frame, chunks[1], app),
        Tab::Sessions => sessions::render(frame, chunks[1], app),
        Tab::Events => events::render(frame, chunks[1], app),
        Tab::Publishers => publishers::render(frame, chunks[1], app),
        Tab::Platforms => platforms::render(frame, chunks[1], app),
        Tab::Resolve => resolve::render(frame, chunks[1], app),
    }

    render_status_bar(frame, chunks[2], app);

    // Popup overlay
    if let Some(popup) = &app.popup {
        render_popup(frame, popup);
    }
}

fn render_tab_bar(frame: &mut Frame, area: Rect, active: Tab) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .enumerate()
        .map(|(i, t)| {
            Line::from(vec![
                Span::styled(
                    format!(" {} ", i + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(t.title()),
                Span::raw(" "),
            ])
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" OA Telemetry "),
        )
        .select(active.index())
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, area);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans = Vec::new();

    // Connection indicators
    if app.has_db() {
        spans.push(Span::styled(" DB", Style::default().fg(Color::Green)));
    } else {
        spans.push(Span::styled(" DB", Style::default().fg(Color::DarkGray)));
    }
    spans.push(Span::raw(" "));
    if app.has_http() {
        spans.push(Span::styled("HTTP", Style::default().fg(Color::Green)));
    } else {
        spans.push(Span::styled("HTTP", Style::default().fg(Color::DarkGray)));
    }
    spans.push(Span::raw("  "));

    // Status message or help hint
    if let Some((msg, _)) = &app.status_message {
        spans.push(Span::styled(msg, Style::default().fg(Color::Yellow)));
    } else {
        spans.push(Span::styled(
            "q:quit  Tab:switch  r:refresh  1-6:tabs",
            Style::default().fg(Color::DarkGray),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_popup(frame: &mut Frame, popup: &crate::app::PopupState) {
    let area = centered_rect(60, 40, frame.area());

    // Clear background
    frame.render_widget(ratatui::widgets::Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" {} ", popup.title));

    let text = Paragraph::new(popup.message.as_str())
        .block(block)
        .style(Style::default().fg(Color::White));

    frame.render_widget(text, area);
}

/// Create a centered rect using percentages of the parent area.
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
