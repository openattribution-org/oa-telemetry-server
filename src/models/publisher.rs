use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Database rows
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Publisher {
    pub id: Uuid,
    pub name: String,
    pub domains: Vec<String>,
    pub api_key_hash: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PlatformKey {
    pub id: Uuid,
    pub name: String,
    pub platform_id: String,
    pub api_key_hash: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Publisher query responses
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct PublisherSummary {
    pub publisher_id: Uuid,
    pub publisher_name: String,
    pub domains: Vec<String>,
    pub total_events: i64,
    pub total_sessions: i64,
    pub events_by_type: Vec<EventTypeCount>,
    pub agents: Vec<AgentBreakdown>,
    pub period_start: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentBreakdown {
    pub platform_id: Option<String>,
    pub agent_id: Option<String>,
    pub event_count: i64,
    pub session_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventTypeCount {
    pub event_type: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublisherEvent {
    pub event_id: Uuid,
    pub session_id: Uuid,
    pub event_type: String,
    pub content_url: Option<String>,
    pub event_timestamp: DateTime<Utc>,
    pub event_data: serde_json::Value,
    pub platform_id: Option<String>,
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublisherUrlMetric {
    pub content_url: String,
    pub total_events: i64,
    pub unique_sessions: i64,
    pub event_types: Vec<EventTypeCount>,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DailyMetricRow {
    pub publisher_id: Uuid,
    pub metric_date: NaiveDate,
    pub domain: String,
    pub event_type: String,
    pub event_count: i64,
    pub unique_sessions: i64,
}

// ---------------------------------------------------------------------------
// Query params
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct PublisherQueryParams {
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaginatedQueryParams {
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub domain: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

#[derive(Debug, Clone, Serialize)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ResolveResponse {
    pub domain: String,
    pub handled: bool,
    pub publisher: Option<ResolvedPublisher>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedPublisher {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WellKnownResponse {
    pub openattribution: WellKnownMeta,
}

#[derive(Debug, Clone, Serialize)]
pub struct WellKnownMeta {
    pub version: String,
    pub server: String,
    pub capabilities: Vec<String>,
    pub registered_domains: Vec<String>,
}
