use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // input
            Constraint::Length(7), // result
            Constraint::Min(0),   // domain index
        ])
        .split(area);

    render_input(frame, chunks[0], app);
    render_result(frame, chunks[1], app);
    render_domain_index(frame, chunks[2], app);
}

fn render_input(frame: &mut Frame, area: Rect, app: &App) {
    let border_color = if app.resolve.input_active {
        Color::Cyan
    } else {
        Color::White
    };

    let cursor = if app.resolve.input_active { "█" } else { "" };
    let display = format!("{}{cursor}", app.resolve.input);

    let hint = if app.resolve.input_active {
        " [Enter to resolve, Esc to cancel] "
    } else {
        " [Enter or / to type] "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Enter URL or domain ")
        .title_bottom(hint);

    let para = Paragraph::new(format!("  {display}")).block(block);
    frame.render_widget(para, area);
}

fn render_result(frame: &mut Frame, area: Rect, app: &App) {
    let lines = if app.resolve.loading {
        vec![Line::from("  Resolving...")]
    } else if let Some(r) = &app.resolve.result {
        let handled_span = if r.handled {
            Span::styled("yes", Style::default().fg(Color::Green))
        } else {
            Span::styled("no", Style::default().fg(Color::Yellow))
        };

        let mut lines = vec![
            Line::from(format!("  Domain:    {}", r.domain)),
            Line::from(vec![Span::raw("  Handled:   "), handled_span]),
        ];

        if let Some(name) = &r.publisher_name {
            let id = r
                .publisher_id
                .map(|u| u.to_string())
                .unwrap_or_default();
            lines.push(Line::from(format!("  Publisher: {name} ({id})")));
        } else {
            lines.push(Line::from(
                "  Publisher: (none — domain not registered)",
            ));
        }

        lines
    } else {
        vec![Line::from(
            "  Enter a URL or domain above to resolve it",
        )]
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Result ");
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_domain_index(frame: &mut Frame, area: Rect, app: &App) {
    let header = Row::new(vec![Cell::from("Domain"), Cell::from("Publisher ID")])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let rows: Vec<Row> = app
        .resolve
        .domain_entries
        .iter()
        .map(|(domain, id)| {
            Row::new(vec![
                Cell::from(domain.clone()),
                Cell::from(id.to_string()),
            ])
        })
        .collect();

    let widths = [Constraint::Min(30), Constraint::Min(38)];

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " Domain Index ({} entries) ",
                app.resolve.domain_entries.len()
            )),
    );

    frame.render_widget(table, area);
}
