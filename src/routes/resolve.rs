use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::domain::extract_domain;
use crate::errors::OaError;
use crate::models::publisher::{ResolveResponse, ResolvedPublisher, WellKnownMeta, WellKnownResponse};
use crate::state::OaState;

#[derive(Debug, Deserialize)]
pub struct ResolveParams {
    pub url: Option<String>,
    pub domain: Option<String>,
}

/// GET /resolve?url=...&domain=...
///
/// Resolve a content URL or domain to its registered publisher.
/// The "DNS resolver for telemetry" endpoint.
pub async fn resolve(
    State(state): State<OaState>,
    Query(params): Query<ResolveParams>,
) -> Result<Json<ResolveResponse>, OaError> {
    let domain = if let Some(url) = &params.url {
        extract_domain(url).ok_or_else(|| OaError::BadRequest("Invalid URL".to_string()))?
    } else if let Some(d) = &params.domain {
        d.strip_prefix("www.")
            .unwrap_or(d)
            .to_lowercase()
    } else {
        return Err(OaError::BadRequest(
            "Provide either 'url' or 'domain' parameter".to_string(),
        ));
    };

    // Check domain index
    if let Some(publisher_id) = state.domain_index.get(&domain).map(|e| *e.value()) {
        // Look up publisher name (check cache or DB)
        // For resolution, we iterate the domain index's source data
        // Since we only have the ID from the index, do a lightweight lookup
        let publisher = sqlx::query_as::<_, PublisherNameRow>(
            "SELECT id, name FROM publishers WHERE id = $1",
        )
        .bind(publisher_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(OaError::from)?;

        if let Some(p) = publisher {
            return Ok(Json(ResolveResponse {
                domain,
                handled: true,
                publisher: Some(ResolvedPublisher {
                    id: p.id,
                    name: p.name,
                }),
            }));
        }
    }

    // Also check parent domain
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() > 2 {
        let parent = parts[1..].join(".");
        if let Some(publisher_id) = state.domain_index.get(&parent).map(|e| *e.value()) {
            let publisher = sqlx::query_as::<_, PublisherNameRow>(
                "SELECT id, name FROM publishers WHERE id = $1",
            )
            .bind(publisher_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(OaError::from)?;

            if let Some(p) = publisher {
                return Ok(Json(ResolveResponse {
                    domain,
                    handled: true,
                    publisher: Some(ResolvedPublisher {
                        id: p.id,
                        name: p.name,
                    }),
                }));
            }
        }
    }

    Ok(Json(ResolveResponse {
        domain,
        handled: false,
        publisher: None,
    }))
}

/// GET /.well-known/openattribution.json
///
/// Server metadata and registered domains for machine-readable discovery.
pub async fn well_known(
    State(state): State<OaState>,
) -> Json<WellKnownResponse> {
    let registered_domains: Vec<String> = state
        .domain_index
        .iter()
        .map(|entry| entry.key().clone())
        .collect();

    Json(WellKnownResponse {
        openattribution: WellKnownMeta {
            version: "0.4".to_string(),
            server: "oa-telemetry-server".to_string(),
            capabilities: vec![
                "sessions".to_string(),
                "events".to_string(),
                "publisher-queries".to_string(),
                "domain-resolution".to_string(),
            ],
            registered_domains,
        },
    })
}

#[derive(Debug, sqlx::FromRow)]
struct PublisherNameRow {
    id: uuid::Uuid,
    name: String,
}
