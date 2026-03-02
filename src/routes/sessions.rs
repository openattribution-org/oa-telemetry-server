use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

use crate::auth::platform::PlatformAuth;
use crate::errors::OaError;
use crate::models::session::{
    BulkSessionRequest, BulkSessionResponse, SessionCreateRequest, SessionEndRequest,
    SessionStartResponse,
};
use crate::services::{events, sessions};
use crate::state::OaState;

/// POST /session/start
pub async fn start_session(
    State(state): State<OaState>,
    PlatformAuth(platform_key): PlatformAuth,
    Json(req): Json<SessionCreateRequest>,
) -> Result<impl IntoResponse, OaError> {
    let mut req = req;
    // Inject platform_id from the authenticated key if not provided
    if req.platform_id.is_none() {
        req.platform_id = Some(platform_key.platform_id.clone());
    }

    let session = sessions::create_session(&state.pool, &req).await?;

    // Cache as active
    state.session_cache.insert(session.id, true).await;

    tracing::info!(
        session_id = %session.id,
        platform_id = %platform_key.platform_id,
        "Session started"
    );

    Ok((
        StatusCode::CREATED,
        Json(SessionStartResponse {
            session_id: session.id.to_string(),
        }),
    ))
}

/// POST /session/end
pub async fn end_session(
    State(state): State<OaState>,
    PlatformAuth(platform_key): PlatformAuth,
    Json(req): Json<SessionEndRequest>,
) -> Result<Json<serde_json::Value>, OaError> {
    let session = sessions::end_session(&state.pool, &req)
        .await?
        .ok_or_else(|| OaError::NotFound("Session not found".to_string()))?;

    // Invalidate cache
    state.session_cache.insert(session.id, false).await;

    tracing::info!(
        session_id = %session.id,
        platform_id = %platform_key.platform_id,
        outcome_type = ?session.outcome_type,
        "Session ended"
    );

    Ok(Json(serde_json::json!({
        "status": "ok",
        "session_id": session.id.to_string()
    })))
}

/// POST /session/bulk
pub async fn bulk_session(
    State(state): State<OaState>,
    PlatformAuth(platform_key): PlatformAuth,
    Json(req): Json<BulkSessionRequest>,
) -> Result<impl IntoResponse, OaError> {
    // Create session from bulk data
    let create_req = SessionCreateRequest {
        initiator_type: req.initiator_type.clone(),
        initiator: req.initiator.clone(),
        content_scope: req.content_scope.clone(),
        manifest_ref: req.manifest_ref.clone(),
        agent_id: req.agent_id.clone(),
        external_session_id: Some(req.session_id.to_string()),
        user_context: req.user_context.clone(),
        prior_session_ids: req.prior_session_ids.iter().map(|u| u.to_string()).collect(),
        platform_id: req.platform_id.clone().or_else(|| Some(platform_key.platform_id.clone())),
        client_type: req.client_type.clone(),
        client_info: req.client_info.clone(),
    };

    let session = sessions::create_session(&state.pool, &create_req).await?;
    let mut response = BulkSessionResponse {
        session_id: session.id.to_string(),
        events_created: 0,
        outcome_recorded: false,
    };

    // Record events
    if !req.events.is_empty() {
        let created = events::create_events(&state.pool, session.id, &req.events).await?;
        response.events_created = created.len();
    }

    // End session with outcome
    if let Some(outcome) = &req.outcome {
        let end_req = SessionEndRequest {
            session_id: session.id.to_string(),
            outcome: outcome.clone(),
        };
        sessions::end_session(&state.pool, &end_req).await?;
        response.outcome_recorded = true;
        state.session_cache.insert(session.id, false).await;
    } else {
        state.session_cache.insert(session.id, true).await;
    }

    tracing::info!(
        session_id = %session.id,
        platform_id = %platform_key.platform_id,
        events_created = response.events_created,
        outcome_recorded = response.outcome_recorded,
        "Bulk session uploaded"
    );

    Ok((StatusCode::CREATED, Json(response)))
}
