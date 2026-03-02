use axum::extract::{Query, State};
use axum::Json;

use crate::auth::publisher::PublisherAuth;
use crate::errors::OaError;
use crate::models::publisher::{
    Paginated, PaginatedQueryParams, PublisherEvent, PublisherQueryParams, PublisherSummary,
    PublisherUrlMetric,
};
use crate::services::publishers;
use crate::state::OaState;

/// GET /publisher/summary
pub async fn publisher_summary(
    State(state): State<OaState>,
    PublisherAuth(publisher): PublisherAuth,
    Query(params): Query<PublisherQueryParams>,
) -> Result<Json<PublisherSummary>, OaError> {
    let summary = publishers::get_publisher_summary(
        &state.pool,
        &publisher,
        params.since,
        params.until,
        params.domain.as_deref(),
    )
    .await?;

    Ok(Json(summary))
}

/// GET /publisher/events
pub async fn publisher_events(
    State(state): State<OaState>,
    PublisherAuth(publisher): PublisherAuth,
    Query(params): Query<PaginatedQueryParams>,
) -> Result<Json<Paginated<PublisherEvent>>, OaError> {
    let events = publishers::get_publisher_events(
        &state.pool,
        &publisher,
        params.since,
        params.until,
        params.domain.as_deref(),
        params.limit,
        params.offset,
    )
    .await?;

    Ok(Json(events))
}

/// GET /publisher/urls
pub async fn publisher_urls(
    State(state): State<OaState>,
    PublisherAuth(publisher): PublisherAuth,
    Query(params): Query<PaginatedQueryParams>,
) -> Result<Json<Paginated<PublisherUrlMetric>>, OaError> {
    let metrics = publishers::get_publisher_url_metrics(
        &state.pool,
        &publisher,
        params.since,
        params.until,
        params.domain.as_deref(),
        params.limit,
        params.offset,
    )
    .await?;

    Ok(Json(metrics))
}
