use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

use crate::auth::platform::PlatformAuth;
use crate::errors::OaError;
use crate::models::click_token::{ClickTokenCreateRequest, ClickTokenCreateResponse};
use crate::services::click_tokens;
use crate::state::OaState;

/// POST /click-tokens
///
/// Agent records a click token when generating a tracked outbound URL.
pub async fn create_click_token(
    State(state): State<OaState>,
    PlatformAuth(_platform_key): PlatformAuth,
    Json(req): Json<ClickTokenCreateRequest>,
) -> Result<impl IntoResponse, OaError> {
    let row = click_tokens::create_click_token(
        &state.pool,
        req.session_id,
        &req.content_url,
        req.token.as_deref(),
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(ClickTokenCreateResponse {
            token: row.token,
            session_id: row.session_id,
            content_url: row.content_url,
            expires_at: row.expires_at,
        }),
    ))
}

/// GET /ctx/{token}
///
/// Public endpoint. A retailer or affiliate network looks up the session
/// context for a click token to see which content was cited.
pub async fn lookup_context(
    State(state): State<OaState>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, OaError> {
    let context = click_tokens::lookup_by_token(&state.pool, &token)
        .await?
        .ok_or_else(|| OaError::NotFound("Click token not found or expired".to_string()))?;

    Ok(Json(context))
}
