use std::time::{Duration, Instant};

use uuid::Uuid;

use oa_telemetry_server::models::event::EventRow;
use oa_telemetry_server::models::publisher::{PlatformKey, Publisher, PublisherSummary};
use oa_telemetry_server::models::session::{SessionSummary, SessionWithEvents};

use crate::backend::Backend;
use crate::tabs::Tab;

/// Top-level application state.
pub struct App {
    pub backend: Backend,
    pub active_tab: Tab,
    pub should_quit: bool,
    pub tick_rate: Duration,

    // Per-tab state
    pub dashboard: DashboardState,
    pub sessions: SessionsState,
    pub events: EventsState,
    pub publishers: PublishersState,
    pub platforms: PlatformsState,
    pub resolve: ResolveState,

    // Status bar
    pub status_message: Option<(String, Instant)>,

    // Popup
    pub popup: Option<PopupState>,
}

impl App {
    pub fn new(backend: Backend, tick_rate: Duration) -> Self {
        Self {
            backend,
            active_tab: Tab::Dashboard,
            should_quit: false,
            tick_rate,
            dashboard: DashboardState::default(),
            sessions: SessionsState::default(),
            events: EventsState::default(),
            publishers: PublishersState::default(),
            platforms: PlatformsState::default(),
            resolve: ResolveState::default(),
            status_message: None,
            popup: None,
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    /// Clear status messages older than 5 seconds.
    pub fn clear_stale_status(&mut self) {
        if let Some((_, t)) = &self.status_message {
            if t.elapsed() > Duration::from_secs(5) {
                self.status_message = None;
            }
        }
    }

    pub fn has_db(&self) -> bool {
        self.backend.db.is_some()
    }

    pub fn has_http(&self) -> bool {
        self.backend.http.is_some()
    }
}

// ---------------------------------------------------------------------------
// Tab state structs
// ---------------------------------------------------------------------------

#[derive(Default)]
#[allow(dead_code)]
pub struct DashboardState {
    pub healthy: Option<bool>,
    pub db_latency_ms: Option<u64>,
    pub session_counts: Option<SessionCounts>,
    pub events_per_minute: Option<i64>,
    pub domain_count: Option<usize>,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SessionCounts {
    pub total: i64,
    pub active: i64,
    pub ended: i64,
    pub conversions: i64,
    pub abandonments: i64,
    pub browses: i64,
}

#[allow(dead_code)]
pub struct SessionsState {
    pub sessions: Vec<SessionSummary>,
    pub selected: usize,
    pub loading: bool,
    pub error: Option<String>,
    pub page: usize,
    pub page_size: i64,

    // Drill-in
    pub detail: Option<SessionWithEvents>,
    pub showing_detail: bool,

    // Filter input
    pub filter_active: bool,
    pub filter_input: String,
}

impl Default for SessionsState {
    fn default() -> Self {
        Self {
            sessions: Vec::new(),
            selected: 0,
            loading: false,
            error: None,
            page: 0,
            page_size: 50,
            detail: None,
            showing_detail: false,
            filter_active: false,
            filter_input: String::new(),
        }
    }
}

#[derive(Default)]
#[allow(dead_code)]
pub struct EventsState {
    pub events: Vec<EventRow>,
    pub selected: usize,
    pub loading: bool,
    pub error: Option<String>,
    pub paused: bool,
}

#[derive(Default)]
#[allow(dead_code)]
pub struct PublishersState {
    pub publishers: Vec<Publisher>,
    pub selected: usize,
    pub loading: bool,
    pub error: Option<String>,

    // Summary drill-in
    pub summary: Option<PublisherSummary>,
    pub showing_summary: bool,

    // Keygen input
    pub keygen_active: bool,
    pub keygen_name: String,
    pub keygen_domains: String,
    pub keygen_field: u8, // 0=name, 1=domains
}

#[derive(Default)]
#[allow(dead_code)]
pub struct PlatformsState {
    pub platforms: Vec<PlatformKey>,
    pub selected: usize,
    pub loading: bool,
    pub error: Option<String>,

    // Keygen input
    pub keygen_active: bool,
    pub keygen_name: String,
    pub keygen_platform_id: String,
    pub keygen_field: u8, // 0=name, 1=platform_id
}

#[derive(Default)]
#[allow(dead_code)]
pub struct ResolveState {
    pub input: String,
    pub input_active: bool,
    pub result: Option<ResolveResult>,
    pub domain_entries: Vec<(String, Uuid)>,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolveResult {
    pub domain: String,
    pub handled: bool,
    pub publisher_name: Option<String>,
    pub publisher_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct PopupState {
    pub title: String,
    pub message: String,
}
