use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use uuid::Uuid;

use crate::auth::platform::PlatformAuth;
use crate::errors::OaError;
use crate::models::event::{EventsCreateRequest, EventsCreatedResponse};
use crate::services::{events, sessions};
use crate::state::OaState;

/// POST /events
pub async fn record_events(
    State(state): State<OaState>,
    PlatformAuth(_platform_key): PlatformAuth,
    Json(req): Json<EventsCreateRequest>,
) -> Result<impl IntoResponse, OaError> {
    let session_id: Uuid = req
        .session_id
        .parse()
        .map_err(|_| OaError::BadRequest(format!("Invalid session_id: {}", req.session_id)))?;

    // Check cache for active session, fall back to DB
    let session_active = if let Some(active) = state.session_cache.get(&session_id).await {
        active
    } else {
        let session = sessions::get_session(&state.pool, session_id)
            .await?
            .ok_or_else(|| OaError::NotFound("Session not found".to_string()))?;
        let active = session.ended_at.is_none();
        state.session_cache.insert(session_id, active).await;
        active
    };

    if !session_active {
        // Re-check from DB in case cache is stale
        let session = sessions::get_session(&state.pool, session_id)
            .await?
            .ok_or_else(|| OaError::NotFound("Session not found".to_string()))?;
        if session.ended_at.is_some() {
            return Err(OaError::BadRequest(
                "Cannot add events to an ended session".to_string(),
            ));
        }
    }

    let created = events::create_events(&state.pool, session_id, &req.events).await?;

    Ok((
        StatusCode::CREATED,
        Json(EventsCreatedResponse {
            status: "ok".to_string(),
            events_created: created.len(),
        }),
    ))
}
