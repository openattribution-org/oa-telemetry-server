use std::time::Instant;

use sqlx::PgPool;
use uuid::Uuid;

use oa_telemetry_server::auth::{generate_raw_key, hash_api_key};
use oa_telemetry_server::models::event::EventRow;
use oa_telemetry_server::models::publisher::{PlatformKey, Publisher};
use oa_telemetry_server::models::session::{SessionSummary, SessionWithEvents};
use oa_telemetry_server::services;

use crate::app::{ResolveResult, SessionCounts};

// ---------------------------------------------------------------------------
// Backend
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Backend {
    pub db: Option<DbBackend>,
    pub http: Option<HttpBackend>,
}

#[derive(Clone)]
pub struct DbBackend {
    pub pool: PgPool,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct HttpBackend {
    pub client: reqwest::Client,
    pub base_url: String,
    pub api_key: Option<String>,
}

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum BackendError {
    #[error("{0}")]
    Db(#[from] sqlx::Error),
    #[error("{0}")]
    Http(#[from] reqwest::Error),
    #[error("Not available in this connection mode")]
    NotAvailable,
    #[error("{0}")]
    Other(String),
}

pub type BackendResult<T> = Result<T, BackendError>;

// ---------------------------------------------------------------------------
// Backend methods — dispatch to DB (preferred) or HTTP
// ---------------------------------------------------------------------------

impl Backend {
    /// Health check. Returns (is_healthy, latency_ms).
    pub async fn health_check(&self) -> BackendResult<(bool, u64)> {
        if let Some(db) = &self.db {
            let start = Instant::now();
            let ok = sqlx::query_scalar::<_, i32>("SELECT 1")
                .fetch_one(&db.pool)
                .await
                .is_ok();
            Ok((ok, start.elapsed().as_millis() as u64))
        } else if let Some(http) = &self.http {
            let start = Instant::now();
            let resp = http
                .client
                .get(format!("{}/ready", http.base_url))
                .send()
                .await?;
            let ok = resp.status().is_success();
            Ok((ok, start.elapsed().as_millis() as u64))
        } else {
            Err(BackendError::NotAvailable)
        }
    }

    /// Aggregate session counts.
    pub async fn session_counts(&self) -> BackendResult<SessionCounts> {
        let db = self.db.as_ref().ok_or(BackendError::NotAvailable)?;
        let row = sqlx::query_as::<_, SessionCountsRow>(
            "SELECT
                COUNT(*)::bigint as total,
                COUNT(*) FILTER (WHERE ended_at IS NULL)::bigint as active,
                COUNT(*) FILTER (WHERE ended_at IS NOT NULL)::bigint as ended,
                COUNT(*) FILTER (WHERE outcome_type = 'conversion')::bigint as conversions,
                COUNT(*) FILTER (WHERE outcome_type = 'abandonment')::bigint as abandonments,
                COUNT(*) FILTER (WHERE outcome_type = 'browse')::bigint as browses
             FROM sessions",
        )
        .fetch_one(&db.pool)
        .await?;
        Ok(SessionCounts {
            total: row.total,
            active: row.active,
            ended: row.ended,
            conversions: row.conversions,
            abandonments: row.abandonments,
            browses: row.browses,
        })
    }

    /// Events recorded in the last minute.
    pub async fn events_per_minute(&self) -> BackendResult<i64> {
        let db = self.db.as_ref().ok_or(BackendError::NotAvailable)?;
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM events WHERE event_timestamp > NOW() - interval '1 minute'",
        )
        .fetch_one(&db.pool)
        .await?;
        Ok(count)
    }

    /// Number of domains in the domain index.
    pub async fn domain_count(&self) -> BackendResult<usize> {
        if let Some(db) = &self.db {
            let publishers = services::publishers::list_active_publishers(&db.pool).await?;
            let count: usize = publishers.iter().map(|p| p.domains.len()).sum();
            Ok(count)
        } else if let Some(http) = &self.http {
            let resp: serde_json::Value = http
                .client
                .get(format!("{}/.well-known/openattribution.json", http.base_url))
                .send()
                .await?
                .json()
                .await?;
            let count = resp["openattribution"]["registered_domains"]
                .as_array()
                .map(|a| a.len())
                .unwrap_or(0);
            Ok(count)
        } else {
            Err(BackendError::NotAvailable)
        }
    }

    /// List sessions with optional filters.
    pub async fn list_sessions(
        &self,
        outcome_type: Option<&str>,
        content_scope: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> BackendResult<Vec<SessionSummary>> {
        if let Some(db) = &self.db {
            let rows = services::sessions::list_sessions(
                &db.pool,
                outcome_type,
                content_scope,
                None,
                None,
                limit,
                offset,
            )
            .await?;
            Ok(rows)
        } else if let Some(http) = &self.http {
            let mut url = format!("{}/internal/sessions?limit={}&offset={}", http.base_url, limit, offset);
            if let Some(ot) = outcome_type {
                url.push_str(&format!("&outcome_type={ot}"));
            }
            if let Some(cs) = content_scope {
                url.push_str(&format!("&content_scope={cs}"));
            }
            let rows: Vec<SessionSummary> = http.client.get(&url).send().await?.json().await?;
            Ok(rows)
        } else {
            Err(BackendError::NotAvailable)
        }
    }

    /// Get session detail with events.
    pub async fn get_session_detail(&self, id: Uuid) -> BackendResult<Option<SessionWithEvents>> {
        if let Some(db) = &self.db {
            let result = services::sessions::get_session_with_events(&db.pool, id).await?;
            Ok(result)
        } else if let Some(http) = &self.http {
            let resp = http
                .client
                .get(format!("{}/internal/sessions/{}", http.base_url, id))
                .send()
                .await?;
            if resp.status().as_u16() == 404 {
                return Ok(None);
            }
            let detail: SessionWithEvents = resp.json().await?;
            Ok(Some(detail))
        } else {
            Err(BackendError::NotAvailable)
        }
    }

    /// Most recent events across all sessions.
    pub async fn recent_events(&self, limit: i64) -> BackendResult<Vec<EventRow>> {
        let db = self.db.as_ref().ok_or(BackendError::NotAvailable)?;
        let rows = sqlx::query_as::<_, EventRow>(
            "SELECT * FROM events ORDER BY event_timestamp DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&db.pool)
        .await?;
        Ok(rows)
    }

    /// List all publishers.
    pub async fn list_publishers(&self) -> BackendResult<Vec<Publisher>> {
        let db = self.db.as_ref().ok_or(BackendError::NotAvailable)?;
        let rows = sqlx::query_as::<_, Publisher>(
            "SELECT * FROM publishers ORDER BY created_at DESC",
        )
        .fetch_all(&db.pool)
        .await?;
        Ok(rows)
    }

    /// List all platform keys.
    pub async fn list_platform_keys(&self) -> BackendResult<Vec<PlatformKey>> {
        let db = self.db.as_ref().ok_or(BackendError::NotAvailable)?;
        let rows = sqlx::query_as::<_, PlatformKey>(
            "SELECT * FROM platform_keys ORDER BY created_at DESC",
        )
        .fetch_all(&db.pool)
        .await?;
        Ok(rows)
    }

    /// Generate a new publisher key. Returns the raw key.
    pub async fn generate_publisher_key(
        &self,
        name: &str,
        domains: Vec<String>,
    ) -> BackendResult<String> {
        let db = self.db.as_ref().ok_or(BackendError::NotAvailable)?;
        let raw_key = generate_raw_key("oat_pub");
        let hash = hash_api_key(&raw_key);
        sqlx::query("INSERT INTO publishers (name, domains, api_key_hash) VALUES ($1, $2, $3)")
            .bind(name)
            .bind(&domains)
            .bind(&hash)
            .execute(&db.pool)
            .await?;
        Ok(raw_key)
    }

    /// Generate a new platform key. Returns the raw key.
    pub async fn generate_platform_key(
        &self,
        name: &str,
        platform_id: &str,
    ) -> BackendResult<String> {
        let db = self.db.as_ref().ok_or(BackendError::NotAvailable)?;
        let raw_key = generate_raw_key("oat_pk");
        let hash = hash_api_key(&raw_key);
        sqlx::query(
            "INSERT INTO platform_keys (name, platform_id, api_key_hash) VALUES ($1, $2, $3)",
        )
        .bind(name)
        .bind(platform_id)
        .bind(&hash)
        .execute(&db.pool)
        .await?;
        Ok(raw_key)
    }

    /// Resolve a domain to a publisher.
    pub async fn resolve_domain(&self, input: &str) -> BackendResult<ResolveResult> {
        if let Some(http) = &self.http {
            let resp: serde_json::Value = http
                .client
                .get(format!("{}/resolve", http.base_url))
                .query(&[("url", input)])
                .send()
                .await?
                .json()
                .await?;
            Ok(ResolveResult {
                domain: resp["domain"].as_str().unwrap_or("").to_string(),
                handled: resp["handled"].as_bool().unwrap_or(false),
                publisher_name: resp["publisher"]["name"].as_str().map(String::from),
                publisher_id: resp["publisher"]["id"]
                    .as_str()
                    .and_then(|s| s.parse().ok()),
            })
        } else if let Some(db) = &self.db {
            // Resolve using domain logic directly
            let domain = oa_telemetry_server::domain::extract_domain(input)
                .unwrap_or_else(|| input.to_string());
            let publishers = services::publishers::list_active_publishers(&db.pool).await?;
            if let Some(pub_) =
                services::publishers::resolve_domain_to_publisher(&publishers, &domain)
            {
                Ok(ResolveResult {
                    domain,
                    handled: true,
                    publisher_name: Some(pub_.name.clone()),
                    publisher_id: Some(pub_.id),
                })
            } else {
                Ok(ResolveResult {
                    domain,
                    handled: false,
                    publisher_name: None,
                    publisher_id: None,
                })
            }
        } else {
            Err(BackendError::NotAvailable)
        }
    }

    /// Get all domain->publisher mappings.
    pub async fn domain_index_entries(&self) -> BackendResult<Vec<(String, Uuid)>> {
        let db = self.db.as_ref().ok_or(BackendError::NotAvailable)?;
        let publishers = services::publishers::list_active_publishers(&db.pool).await?;
        let mut entries = Vec::new();
        for p in &publishers {
            for d in &p.domains {
                entries.push((d.clone(), p.id));
            }
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(entries)
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SessionCountsRow {
    total: i64,
    active: i64,
    ended: i64,
    conversions: i64,
    abandonments: i64,
    browses: i64,
}
