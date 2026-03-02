use axum::extract::FromRef;
use axum::routing::{get, post};
use axum::Router;

use crate::routes;
use crate::state::OaState;

/// Write routes: session start/end/bulk, events.
/// Protected by `PlatformAuth`.
pub fn write_routes<S>() -> Router<S>
where
    OaState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/session/start", post(routes::sessions::start_session))
        .route("/session/end", post(routes::sessions::end_session))
        .route("/session/bulk", post(routes::sessions::bulk_session))
        .route("/events", post(routes::events::record_events))
}

/// Publisher query routes.
/// Protected by `PublisherAuth`.
pub fn publisher_routes<S>() -> Router<S>
where
    OaState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/publisher/summary", get(routes::publisher::publisher_summary))
        .route("/publisher/events", get(routes::publisher::publisher_events))
        .route("/publisher/urls", get(routes::publisher::publisher_urls))
}

/// Internal routes for attribution systems.
/// No auth (meant for service-to-service access).
pub fn internal_routes<S>() -> Router<S>
where
    OaState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/internal/sessions/{id}", get(routes::internal::get_session))
        .route("/internal/sessions", get(routes::internal::list_sessions))
        .route(
            "/internal/sessions/by-external-id/{external_id}",
            get(routes::internal::get_session_by_external_id),
        )
}

/// Resolution routes: domain lookup + .well-known.
/// No auth (public discovery).
pub fn resolve_routes<S>() -> Router<S>
where
    OaState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/resolve", get(routes::resolve::resolve))
        .route(
            "/.well-known/openattribution.json",
            get(routes::resolve::well_known),
        )
}

/// Health check routes.
pub fn health_routes<S>() -> Router<S>
where
    OaState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/health", get(routes::health::health))
        .route("/ready", get(routes::health::ready))
}

/// All routes combined. Use this for standalone deployment.
pub fn router<S>() -> Router<S>
where
    OaState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .merge(write_routes())
        .merge(publisher_routes())
        .merge(internal_routes())
        .merge(resolve_routes())
        .merge(health_routes())
}
