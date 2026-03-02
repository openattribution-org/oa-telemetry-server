use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::auth::hash_api_key;
use crate::models::publisher::Publisher;
use crate::services::publishers;
use crate::state::OaState;

/// Extractor that validates a publisher API key from the `X-API-Key` header.
///
/// Uses a moka cache to avoid repeated database lookups.
#[derive(Debug, Clone)]
pub struct PublisherAuth(pub Publisher);

#[derive(Debug)]
pub enum PublisherAuthError {
    MissingKey,
    InvalidKey,
    InternalError,
}

impl IntoResponse for PublisherAuthError {
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

impl<S> FromRequestParts<S> for PublisherAuth
where
    OaState: axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = PublisherAuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let oa_state = OaState::from_ref(state);

        let key = parts
            .headers
            .get("X-API-Key")
            .and_then(|v| v.to_str().ok())
            .ok_or(PublisherAuthError::MissingKey)?;

        let hash = hash_api_key(key);

        // Check cache first
        if let Some(cached) = oa_state.publisher_cache.get(&hash).await {
            tracing::info!(
                publisher_id = %cached.id,
                publisher_name = %cached.name,
                "Publisher key validated (cached)"
            );
            return Ok(Self(cached));
        }

        // Cache miss - query database
        let publisher = publishers::get_publisher_by_hash(&oa_state.pool, &hash)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Publisher key lookup failed");
                PublisherAuthError::InternalError
            })?
            .ok_or(PublisherAuthError::InvalidKey)?;

        tracing::info!(
            publisher_id = %publisher.id,
            publisher_name = %publisher.name,
            "Publisher key validated"
        );

        oa_state
            .publisher_cache
            .insert(hash, publisher.clone())
            .await;

        Ok(Self(publisher))
    }
}

impl std::ops::Deref for PublisherAuth {
    type Target = Publisher;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
