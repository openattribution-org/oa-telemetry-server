use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::auth::hash_api_key;
use crate::models::publisher::PlatformKey;
use crate::services::publishers;
use crate::state::OaState;

/// Extractor that validates a platform API key from the `X-API-Key` header.
///
/// Uses a moka cache to avoid repeated database lookups.
#[derive(Debug, Clone)]
pub struct PlatformAuth(pub PlatformKey);

#[derive(Debug)]
pub enum PlatformAuthError {
    MissingKey,
    InvalidKey,
    InternalError,
}

impl IntoResponse for PlatformAuthError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            Self::MissingKey => (StatusCode::UNAUTHORIZED, "Missing X-API-Key header"),
            Self::InvalidKey => (StatusCode::UNAUTHORIZED, "Invalid API key"),
            Self::InternalError => (StatusCode::INTERNAL_SERVER_ERROR, "Auth lookup failed"),
        };
        let body = serde_json::json!({ "error": msg });
        (status, axum::Json(body)).into_response()
    }
}

impl<S> FromRequestParts<S> for PlatformAuth
where
    OaState: axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = PlatformAuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let oa_state = OaState::from_ref(state);

        let key = parts
            .headers
            .get("X-API-Key")
            .and_then(|v| v.to_str().ok())
            .ok_or(PlatformAuthError::MissingKey)?;

        let hash = hash_api_key(key);

        // Check cache first
        if let Some(cached) = oa_state.platform_cache.get(&hash).await {
            tracing::info!(
                platform_id = %cached.platform_id,
                key_name = %cached.name,
                "Platform key validated (cached)"
            );
            return Ok(Self(cached));
        }

        // Cache miss - query database
        let platform_key = publishers::get_platform_key_by_hash(&oa_state.pool, &hash)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Platform key lookup failed");
                PlatformAuthError::InternalError
            })?
            .ok_or(PlatformAuthError::InvalidKey)?;

        tracing::info!(
            platform_id = %platform_key.platform_id,
            key_name = %platform_key.name,
            "Platform key validated"
        );

        oa_state
            .platform_cache
            .insert(hash, platform_key.clone())
            .await;

        Ok(Self(platform_key))
    }
}

impl std::ops::Deref for PlatformAuth {
    type Target = PlatformKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
