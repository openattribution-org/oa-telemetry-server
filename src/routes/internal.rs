use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::OaError;
use crate::models::session::{SessionRow, SessionSummary, SessionWithEvents};
use crate::services::sessions;
use crate::state::OaState;

#[derive(Debug, Deserialize)]
pub struct ListSessionsParams {
    pub outcome_type: Option<String>,
    pub content_scope: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

/// GET /internal/sessions/{id}
pub async fn get_session(
    State(state): State<OaState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<SessionWithEvents>, OaError> {
    let session = sessions::get_session_with_events(&state.pool, session_id)
        .await?
        .ok_or_else(|| OaError::NotFound("Session not found".to_string()))?;

    Ok(Json(session))
}

/// GET /internal/sessions
pub async fn list_sessions(
    State(state): State<OaState>,
    Query(params): Query<ListSessionsParams>,
) -> Result<Json<Vec<SessionSummary>>, OaError> {
    let results = sessions::list_sessions(
        &state.pool,
        params.outcome_type.as_deref(),
        params.content_scope.as_deref(),
        params.since,
        params.until,
        params.limit,
        params.offset,
    )
    .await?;

    Ok(Json(results))
}

/// GET /internal/sessions/by-external-id/{external_id}
pub async fn get_session_by_external_id(
    State(state): State<OaState>,
    Path(external_id): Path<String>,
) -> Result<Json<SessionRow>, OaError> {
    let session = sessions::get_session_by_external_id(&state.pool, &external_id)
        .await?
        .ok_or_else(|| OaError::NotFound("Session not found".to_string()))?;

    Ok(Json(session))
}
