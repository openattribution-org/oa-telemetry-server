use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    if app.platforms.keygen_active {
        render_keygen(frame, area, app);
        return;
    }

    if !app.has_db() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Platforms ");
        let msg = Paragraph::new("  Requires database connection (--database-url)")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("Name"),
        Cell::from("Platform ID"),
        Cell::from("Active"),
        Cell::from("Created"),
    ])
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .platforms
        .platforms
        .iter()
        .map(|p| {
            let active = if p.active { "yes" } else { "no" };
            Row::new(vec![
                Cell::from(p.name.clone()),
                Cell::from(p.platform_id.clone()),
                Cell::from(active),
                Cell::from(p.created_at.format("%Y-%m-%d").to_string()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(16),
        Constraint::Min(20),
        Constraint::Length(8),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Platform Keys ({}) ",
                    app.platforms.platforms.len()
                ))
                .title_bottom(" g:keygen  j/k:navigate "),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = TableState::default().with_selected(Some(app.platforms.selected));
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_keygen(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Generate Platform Key [Esc to cancel] ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(inner);

    let name_style = if app.platforms.keygen_field == 0 {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };
    let pid_style = if app.platforms.keygen_field == 1 {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    frame.render_widget(
        Paragraph::new("Name:").style(name_style),
        chunks[0],
    );
    let name_display = format!(
        "{}{}",
        app.platforms.keygen_name,
        if app.platforms.keygen_field == 0 { "█" } else { "" }
    );
    frame.render_widget(Paragraph::new(name_display), chunks[1]);

    frame.render_widget(
        Paragraph::new("Platform ID:").style(pid_style),
        chunks[3],
    );
    let pid_display = format!(
        "{}{}",
        app.platforms.keygen_platform_id,
        if app.platforms.keygen_field == 1 { "█" } else { "" }
    );
    frame.render_widget(Paragraph::new(pid_display), chunks[4]);

    frame.render_widget(
        Paragraph::new("Tab to switch fields, Enter to generate")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[6],
    );
}
