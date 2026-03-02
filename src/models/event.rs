use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// API input (matches SDK TelemetryEvent)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct TelemetryEventInput {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub content_url: Option<String>,
    pub product_id: Option<Uuid>,
    pub turn: Option<serde_json::Value>,
    #[serde(default)]
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventsCreateRequest {
    pub session_id: String,
    pub events: Vec<TelemetryEventInput>,
}

// ---------------------------------------------------------------------------
// Database row
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct EventRow {
    pub id: Uuid,
    pub session_id: Uuid,
    pub event_type: String,
    pub content_url: Option<String>,
    pub product_id: Option<Uuid>,
    pub turn_data: Option<serde_json::Value>,
    pub event_data: serde_json::Value,
    pub event_timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// API responses
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct EventsCreatedResponse {
    pub status: String,
    pub events_created: usize,
}
