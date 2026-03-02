use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// API input
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ClickTokenCreateRequest {
    pub session_id: Uuid,
    pub content_url: String,
    /// Optional token. If not provided, the server generates one.
    pub token: Option<String>,
}

// ---------------------------------------------------------------------------
// API responses
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ClickTokenCreateResponse {
    pub token: String,
    pub session_id: Uuid,
    pub content_url: String,
    pub expires_at: DateTime<Utc>,
}

/// Session context returned when a landing page looks up a click token.
#[derive(Debug, Clone, Serialize)]
pub struct SessionContextResponse {
    pub session_id: Uuid,
    pub started_at: DateTime<Utc>,
    /// The specific URL that was clicked out to.
    pub click_content_url: String,
    /// All content URLs cited in the session.
    pub content_urls_cited: Vec<String>,
    /// All content URLs retrieved in the session.
    pub content_urls_retrieved: Vec<String>,
}

// ---------------------------------------------------------------------------
// Database row
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ClickTokenRow {
    pub id: Uuid,
    pub token: String,
    pub session_id: Uuid,
    pub content_url: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
