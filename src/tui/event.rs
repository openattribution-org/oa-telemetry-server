use std::io::Stdout;

use crossterm::event::{Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures::StreamExt;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio::sync::mpsc;
use tokio::time;
use uuid::Uuid;

use oa_telemetry_server::models::event::EventRow;
use oa_telemetry_server::models::publisher::{PlatformKey, Publisher};
use oa_telemetry_server::models::session::{SessionSummary, SessionWithEvents};

use crate::app::{App, PopupState, ResolveResult, SessionCounts};
use crate::backend::Backend;
use crate::tabs::Tab;
use crate::ui;

// ---------------------------------------------------------------------------
// Data payloads from background tasks
// ---------------------------------------------------------------------------

pub enum DataPayload {
    Health(bool, u64),
    SessionCounts(SessionCounts),
    EventsPerMinute(i64),
    DomainCount(usize),
    SessionList(Vec<SessionSummary>),
    SessionDetail(Option<SessionWithEvents>),
    RecentEvents(Vec<EventRow>),
    Publishers(Vec<Publisher>),
    PlatformKeys(Vec<PlatformKey>),
    KeygenResult(String),
    Resolve(ResolveResult),
    DomainIndex(Vec<(String, Uuid)>),
    Error(String),
}

// ---------------------------------------------------------------------------
// Event loop
// ---------------------------------------------------------------------------

pub async fn run_event_loop(
    mut app: App,
    mut terminal: Terminal<CrosstermBackend<Stdout>>,
) -> color_eyre::Result<()> {
    let mut event_stream = EventStream::new();
    let mut tick_interval = time::interval(app.tick_rate);
    let (tx, mut rx) = mpsc::channel::<DataPayload>(64);

    // Initial data load
    request_dashboard_data(&app.backend, tx.clone());

    loop {
        // Render
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        app.clear_stale_status();

        // Wait for next event
        tokio::select! {
            Some(Ok(event)) = event_stream.next() => {
                handle_terminal_event(&mut app, event, tx.clone());
            }
            _ = tick_interval.tick() => {
                handle_tick(&mut app, tx.clone());
            }
            Some(payload) = rx.recv() => {
                handle_data(&mut app, payload);
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Terminal event handling
// ---------------------------------------------------------------------------

fn handle_terminal_event(app: &mut App, event: CrosstermEvent, tx: mpsc::Sender<DataPayload>) {
    let CrosstermEvent::Key(key) = event else {
        return;
    };

    // Popup dismissal
    if app.popup.is_some() {
        if matches!(key.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')) {
            app.popup = None;
        }
        return;
    }

    // Text input modes
    if handle_input_mode(app, key, tx.clone()) {
        return;
    }

    // Global keys
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Tab => {
            app.active_tab = app.active_tab.next();
            request_tab_data(app, tx);
        }
        KeyCode::BackTab => {
            app.active_tab = app.active_tab.prev();
            request_tab_data(app, tx);
        }
        KeyCode::Char(c @ '1'..='6') => {
            let idx = (c as u8 - b'1') as usize;
            app.active_tab = Tab::from_index(idx);
            request_tab_data(app, tx);
        }
        KeyCode::Char('r') => {
            request_tab_data(app, tx);
            app.set_status("Refreshing...");
        }
        _ => {
            handle_tab_key(app, key, tx);
        }
    }
}

/// Handle keys when a text input field is active. Returns true if consumed.
fn handle_input_mode(app: &mut App, key: KeyEvent, tx: mpsc::Sender<DataPayload>) -> bool {
    // Resolve tab input
    if app.active_tab == Tab::Resolve && app.resolve.input_active {
        match key.code {
            KeyCode::Esc => {
                app.resolve.input_active = false;
            }
            KeyCode::Enter => {
                app.resolve.input_active = false;
                let input = app.resolve.input.clone();
                if !input.is_empty() {
                    let backend = app.backend.clone();
                    tokio::spawn(async move {
                        match backend.resolve_domain(&input).await {
                            Ok(result) => {
                                let _ = tx.send(DataPayload::Resolve(result)).await;
                            }
                            Err(e) => {
                                let _ = tx.send(DataPayload::Error(e.to_string())).await;
                            }
                        }
                    });
                    app.resolve.loading = true;
                }
            }
            KeyCode::Backspace => {
                app.resolve.input.pop();
            }
            KeyCode::Char(c) => {
                app.resolve.input.push(c);
            }
            _ => {}
        }
        return true;
    }

    // Publisher keygen
    if app.active_tab == Tab::Publishers && app.publishers.keygen_active {
        match key.code {
            KeyCode::Esc => {
                app.publishers.keygen_active = false;
            }
            KeyCode::Tab => {
                app.publishers.keygen_field = (app.publishers.keygen_field + 1) % 2;
            }
            KeyCode::Enter => {
                let name = app.publishers.keygen_name.clone();
                let domains_str = app.publishers.keygen_domains.clone();
                if !name.is_empty() && !domains_str.is_empty() {
                    let domains: Vec<String> =
                        domains_str.split(',').map(|s| s.trim().to_string()).collect();
                    let backend = app.backend.clone();
                    tokio::spawn(async move {
                        match backend.generate_publisher_key(&name, domains).await {
                            Ok(key) => {
                                let _ = tx.send(DataPayload::KeygenResult(key)).await;
                            }
                            Err(e) => {
                                let _ = tx.send(DataPayload::Error(e.to_string())).await;
                            }
                        }
                    });
                    app.publishers.keygen_active = false;
                    app.publishers.keygen_name.clear();
                    app.publishers.keygen_domains.clear();
                }
            }
            KeyCode::Backspace => {
                if app.publishers.keygen_field == 0 {
                    app.publishers.keygen_name.pop();
                } else {
                    app.publishers.keygen_domains.pop();
                }
            }
            KeyCode::Char(c) => {
                if app.publishers.keygen_field == 0 {
                    app.publishers.keygen_name.push(c);
                } else {
                    app.publishers.keygen_domains.push(c);
                }
            }
            _ => {}
        }
        return true;
    }

    // Platform keygen
    if app.active_tab == Tab::Platforms && app.platforms.keygen_active {
        match key.code {
            KeyCode::Esc => {
                app.platforms.keygen_active = false;
            }
            KeyCode::Tab => {
                app.platforms.keygen_field = (app.platforms.keygen_field + 1) % 2;
            }
            KeyCode::Enter => {
                let name = app.platforms.keygen_name.clone();
                let platform_id = app.platforms.keygen_platform_id.clone();
                if !name.is_empty() && !platform_id.is_empty() {
                    let backend = app.backend.clone();
                    tokio::spawn(async move {
                        match backend.generate_platform_key(&name, &platform_id).await {
                            Ok(key) => {
                                let _ = tx.send(DataPayload::KeygenResult(key)).await;
                            }
                            Err(e) => {
                                let _ = tx.send(DataPayload::Error(e.to_string())).await;
                            }
                        }
                    });
                    app.platforms.keygen_active = false;
                    app.platforms.keygen_name.clear();
                    app.platforms.keygen_platform_id.clear();
                }
            }
            KeyCode::Backspace => {
                if app.platforms.keygen_field == 0 {
                    app.platforms.keygen_name.pop();
                } else {
                    app.platforms.keygen_platform_id.pop();
                }
            }
            KeyCode::Char(c) => {
                if app.platforms.keygen_field == 0 {
                    app.platforms.keygen_name.push(c);
                } else {
                    app.platforms.keygen_platform_id.push(c);
                }
            }
            _ => {}
        }
        return true;
    }

    false
}

/// Handle per-tab keys (not in input mode).
fn handle_tab_key(app: &mut App, key: KeyEvent, tx: mpsc::Sender<DataPayload>) {
    match app.active_tab {
        Tab::Sessions => match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                app.sessions.selected = app.sessions.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.sessions.selected + 1 < app.sessions.sessions.len() {
                    app.sessions.selected += 1;
                }
            }
            KeyCode::Enter => {
                if app.sessions.showing_detail {
                    return;
                }
                if let Some(s) = app.sessions.sessions.get(app.sessions.selected) {
                    let id = s.id;
                    let backend = app.backend.clone();
                    tokio::spawn(async move {
                        match backend.get_session_detail(id).await {
                            Ok(detail) => {
                                let _ = tx.send(DataPayload::SessionDetail(detail)).await;
                            }
                            Err(e) => {
                                let _ = tx.send(DataPayload::Error(e.to_string())).await;
                            }
                        }
                    });
                    app.sessions.showing_detail = true;
                }
            }
            KeyCode::Esc => {
                app.sessions.showing_detail = false;
                app.sessions.detail = None;
            }
            KeyCode::Char('n') => {
                app.sessions.page += 1;
                app.sessions.selected = 0;
                request_sessions(app, tx);
            }
            KeyCode::Char('p') if app.sessions.page > 0 => {
                app.sessions.page -= 1;
                app.sessions.selected = 0;
                request_sessions(app, tx);
            }
            _ => {}
        },
        Tab::Events => match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                app.events.selected = app.events.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.events.selected + 1 < app.events.events.len() {
                    app.events.selected += 1;
                }
            }
            KeyCode::Char(' ') => {
                app.events.paused = !app.events.paused;
                app.set_status(if app.events.paused {
                    "Event feed paused"
                } else {
                    "Event feed resumed"
                });
            }
            _ => {}
        },
        Tab::Publishers => match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                app.publishers.selected = app.publishers.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.publishers.selected + 1 < app.publishers.publishers.len() {
                    app.publishers.selected += 1;
                }
            }
            KeyCode::Enter => {
                // TODO: publisher summary drill-in
            }
            KeyCode::Esc => {
                app.publishers.showing_summary = false;
                app.publishers.summary = None;
            }
            KeyCode::Char('g') if app.has_db() => {
                app.publishers.keygen_active = true;
                app.publishers.keygen_field = 0;
            }
            _ => {}
        },
        Tab::Platforms => match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                app.platforms.selected = app.platforms.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.platforms.selected + 1 < app.platforms.platforms.len() {
                    app.platforms.selected += 1;
                }
            }
            KeyCode::Char('g') if app.has_db() => {
                app.platforms.keygen_active = true;
                app.platforms.keygen_field = 0;
            }
            _ => {}
        },
        Tab::Resolve => match key.code {
            KeyCode::Enter | KeyCode::Char('/') | KeyCode::Char('i') => {
                app.resolve.input_active = true;
            }
            _ => {}
        },
        Tab::Dashboard => {}
    }
}

// ---------------------------------------------------------------------------
// Tick handling
// ---------------------------------------------------------------------------

fn handle_tick(app: &mut App, tx: mpsc::Sender<DataPayload>) {
    match app.active_tab {
        Tab::Dashboard => {
            request_dashboard_data(&app.backend, tx);
        }
        Tab::Events if !app.events.paused => {
            request_events(&app.backend, tx);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Data response handling
// ---------------------------------------------------------------------------

fn handle_data(app: &mut App, payload: DataPayload) {
    match payload {
        DataPayload::Health(ok, ms) => {
            app.dashboard.healthy = Some(ok);
            app.dashboard.db_latency_ms = Some(ms);
        }
        DataPayload::SessionCounts(counts) => {
            app.dashboard.session_counts = Some(counts);
        }
        DataPayload::EventsPerMinute(n) => {
            app.dashboard.events_per_minute = Some(n);
        }
        DataPayload::DomainCount(n) => {
            app.dashboard.domain_count = Some(n);
        }
        DataPayload::SessionList(sessions) => {
            app.sessions.sessions = sessions;
            app.sessions.loading = false;
            if app.sessions.selected >= app.sessions.sessions.len() && !app.sessions.sessions.is_empty()
            {
                app.sessions.selected = app.sessions.sessions.len() - 1;
            }
        }
        DataPayload::SessionDetail(detail) => {
            app.sessions.detail = detail;
        }
        DataPayload::RecentEvents(events) => {
            app.events.events = events;
            app.events.loading = false;
        }
        DataPayload::Publishers(pubs) => {
            app.publishers.publishers = pubs;
            app.publishers.loading = false;
        }
        DataPayload::PlatformKeys(keys) => {
            app.platforms.platforms = keys;
            app.platforms.loading = false;
        }
        DataPayload::KeygenResult(key) => {
            app.popup = Some(PopupState {
                title: "Key Generated".to_string(),
                message: format!("Save this key now — it will not be shown again:\n\n{key}"),
            });
            // Refresh the relevant list
        }
        DataPayload::Resolve(result) => {
            app.resolve.result = Some(result);
            app.resolve.loading = false;
        }
        DataPayload::DomainIndex(entries) => {
            app.resolve.domain_entries = entries;
        }
        DataPayload::Error(msg) => {
            app.set_status(format!("Error: {msg}"));
            // Clear loading states
            app.dashboard.loading = false;
            app.sessions.loading = false;
            app.events.loading = false;
            app.publishers.loading = false;
            app.platforms.loading = false;
            app.resolve.loading = false;
        }
    }
}

// ---------------------------------------------------------------------------
// Data request helpers
// ---------------------------------------------------------------------------

fn request_dashboard_data(backend: &Backend, tx: mpsc::Sender<DataPayload>) {
    let b = backend.clone();
    let tx2 = tx.clone();
    tokio::spawn(async move {
        if let Ok((ok, ms)) = b.health_check().await {
            let _ = tx2.send(DataPayload::Health(ok, ms)).await;
        }
    });

    let b = backend.clone();
    let tx2 = tx.clone();
    tokio::spawn(async move {
        if let Ok(counts) = b.session_counts().await {
            let _ = tx2.send(DataPayload::SessionCounts(counts)).await;
        }
    });

    let b = backend.clone();
    let tx2 = tx.clone();
    tokio::spawn(async move {
        if let Ok(n) = b.events_per_minute().await {
            let _ = tx2.send(DataPayload::EventsPerMinute(n)).await;
        }
    });

    let b = backend.clone();
    let tx2 = tx;
    tokio::spawn(async move {
        if let Ok(n) = b.domain_count().await {
            let _ = tx2.send(DataPayload::DomainCount(n)).await;
        }
    });
}

fn request_tab_data(app: &App, tx: mpsc::Sender<DataPayload>) {
    match app.active_tab {
        Tab::Dashboard => request_dashboard_data(&app.backend, tx),
        Tab::Sessions => request_sessions(app, tx),
        Tab::Events => request_events(&app.backend, tx),
        Tab::Publishers => request_publishers(&app.backend, tx),
        Tab::Platforms => request_platforms(&app.backend, tx),
        Tab::Resolve => request_domain_index(&app.backend, tx),
    }
}

fn request_sessions(app: &App, tx: mpsc::Sender<DataPayload>) {
    let backend = app.backend.clone();
    let limit = app.sessions.page_size;
    let offset = app.sessions.page as i64 * limit;
    tokio::spawn(async move {
        match backend.list_sessions(None, None, limit, offset).await {
            Ok(sessions) => {
                let _ = tx.send(DataPayload::SessionList(sessions)).await;
            }
            Err(e) => {
                let _ = tx.send(DataPayload::Error(e.to_string())).await;
            }
        }
    });
}

fn request_events(backend: &Backend, tx: mpsc::Sender<DataPayload>) {
    let b = backend.clone();
    tokio::spawn(async move {
        match b.recent_events(100).await {
            Ok(events) => {
                let _ = tx.send(DataPayload::RecentEvents(events)).await;
            }
            Err(e) => {
                let _ = tx.send(DataPayload::Error(e.to_string())).await;
            }
        }
    });
}

fn request_publishers(backend: &Backend, tx: mpsc::Sender<DataPayload>) {
    let b = backend.clone();
    tokio::spawn(async move {
        match b.list_publishers().await {
            Ok(pubs) => {
                let _ = tx.send(DataPayload::Publishers(pubs)).await;
            }
            Err(e) => {
                let _ = tx.send(DataPayload::Error(e.to_string())).await;
            }
        }
    });
}

fn request_platforms(backend: &Backend, tx: mpsc::Sender<DataPayload>) {
    let b = backend.clone();
    tokio::spawn(async move {
        match b.list_platform_keys().await {
            Ok(keys) => {
                let _ = tx.send(DataPayload::PlatformKeys(keys)).await;
            }
            Err(e) => {
                let _ = tx.send(DataPayload::Error(e.to_string())).await;
            }
        }
    });
}

fn request_domain_index(backend: &Backend, tx: mpsc::Sender<DataPayload>) {
    let b = backend.clone();
    tokio::spawn(async move {
        match b.domain_index_entries().await {
            Ok(entries) => {
                let _ = tx.send(DataPayload::DomainIndex(entries)).await;
            }
            Err(e) => {
                let _ = tx.send(DataPayload::Error(e.to_string())).await;
            }
        }
    });
}
