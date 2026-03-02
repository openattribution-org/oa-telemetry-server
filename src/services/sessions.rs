use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::event::EventRow;
use crate::models::session::{
    SessionCreateRequest, SessionEndRequest, SessionRow, SessionSummary, SessionWithEvents,
};

/// Create a new telemetry session.
pub async fn create_session(
    pool: &PgPool,
    req: &SessionCreateRequest,
) -> Result<SessionRow, sqlx::Error> {
    // Parse prior_session_ids, skipping invalid UUIDs
    let prior: Vec<Uuid> = req
        .prior_session_ids
        .iter()
        .filter_map(|s| s.parse::<Uuid>().ok())
        .collect();

    sqlx::query_as::<_, SessionRow>(
        r"INSERT INTO sessions (
            initiator_type, initiator,
            content_scope, manifest_ref,
            agent_id, external_session_id, prior_session_ids,
            user_context, platform_id, client_type, client_info
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING *",
    )
    .bind(&req.initiator_type)
    .bind(&req.initiator)
    .bind(&req.content_scope)
    .bind(&req.manifest_ref)
    .bind(&req.agent_id)
    .bind(&req.external_session_id)
    .bind(&prior)
    .bind(&req.user_context)
    .bind(&req.platform_id)
    .bind(&req.client_type)
    .bind(&req.client_info)
    .fetch_one(pool)
    .await
}

/// Get a session by ID.
pub async fn get_session(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Option<SessionRow>, sqlx::Error> {
    sqlx::query_as::<_, SessionRow>("SELECT * FROM sessions WHERE id = $1")
        .bind(session_id)
        .fetch_optional(pool)
        .await
}

/// Get a session by external session ID (most recent).
pub async fn get_session_by_external_id(
    pool: &PgPool,
    external_id: &str,
) -> Result<Option<SessionRow>, sqlx::Error> {
    sqlx::query_as::<_, SessionRow>(
        "SELECT * FROM sessions WHERE external_session_id = $1 ORDER BY started_at DESC LIMIT 1",
    )
    .bind(external_id)
    .fetch_optional(pool)
    .await
}

/// Get a session with all its events.
pub async fn get_session_with_events(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Option<SessionWithEvents>, sqlx::Error> {
    let Some(session) = get_session(pool, session_id).await? else {
        return Ok(None);
    };

    let events = sqlx::query_as::<_, EventRow>(
        "SELECT * FROM events WHERE session_id = $1 ORDER BY event_timestamp ASC",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;

    Ok(Some(SessionWithEvents { session, events }))
}

/// End a session with outcome.
pub async fn end_session(
    pool: &PgPool,
    req: &SessionEndRequest,
) -> Result<Option<SessionRow>, sqlx::Error> {
    let session_id: Uuid = req
        .session_id
        .parse()
        .map_err(|_| sqlx::Error::Protocol("Invalid session_id UUID".to_string()))?;

    let outcome_value = serde_json::to_value(&req.outcome).unwrap_or_default();

    sqlx::query_as::<_, SessionRow>(
        r"UPDATE sessions
        SET ended_at = NOW(),
            outcome_type = $1,
            outcome_value = $2
        WHERE id = $3
        RETURNING *",
    )
    .bind(&req.outcome.outcome_type)
    .bind(&outcome_value)
    .bind(session_id)
    .fetch_optional(pool)
    .await
}

/// List sessions with filters (for attribution systems).
#[allow(clippy::too_many_arguments)]
pub async fn list_sessions(
    pool: &PgPool,
    outcome_type: Option<&str>,
    content_scope: Option<&str>,
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
    limit: i64,
    offset: i64,
) -> Result<Vec<SessionSummary>, sqlx::Error> {
    // Build dynamic query
    let mut query = String::from(
        "SELECT id, content_scope, external_session_id, outcome_type, started_at, ended_at
         FROM sessions WHERE 1=1",
    );
    let mut param_idx = 1u32;

    // We build the query dynamically and use raw SQL with positional params.
    // sqlx doesn't have a query builder, so we track param positions manually.
    struct Params {
        outcome_type: Option<String>,
        content_scope: Option<String>,
        since: Option<DateTime<Utc>>,
        until: Option<DateTime<Utc>>,
        limit: i64,
        offset: i64,
    }

    let params = Params {
        outcome_type: outcome_type.map(String::from),
        content_scope: content_scope.map(String::from),
        since,
        until,
        limit,
        offset,
    };

    if params.outcome_type.is_some() {
        query.push_str(&format!(" AND outcome_type = ${param_idx}"));
        param_idx += 1;
    }
    if params.content_scope.is_some() {
        query.push_str(&format!(" AND content_scope = ${param_idx}"));
        param_idx += 1;
    }
    if params.since.is_some() {
        query.push_str(&format!(" AND ended_at >= ${param_idx}"));
        param_idx += 1;
    }
    if params.until.is_some() {
        query.push_str(&format!(" AND ended_at <= ${param_idx}"));
        param_idx += 1;
    }

    query.push_str(&format!(
        " ORDER BY started_at DESC LIMIT ${param_idx} OFFSET ${}",
        param_idx + 1
    ));

    // Bind dynamically
    let mut q = sqlx::query_as::<_, SessionSummary>(&query);
    if let Some(ref v) = params.outcome_type {
        q = q.bind(v);
    }
    if let Some(ref v) = params.content_scope {
        q = q.bind(v);
    }
    if let Some(ref v) = params.since {
        q = q.bind(v);
    }
    if let Some(ref v) = params.until {
        q = q.bind(v);
    }
    q = q.bind(params.limit).bind(params.offset);

    q.fetch_all(pool).await
}
