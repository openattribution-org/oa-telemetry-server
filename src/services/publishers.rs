use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::publisher::{
    AgentBreakdown, EventTypeCount, Paginated, PlatformKey, Publisher, PublisherEvent,
    PublisherSummary, PublisherUrlMetric,
};

/// Look up a platform key by its SHA-256 hash.
pub async fn get_platform_key_by_hash(
    pool: &PgPool,
    hash: &str,
) -> Result<Option<PlatformKey>, sqlx::Error> {
    sqlx::query_as::<_, PlatformKey>(
        "SELECT * FROM platform_keys WHERE api_key_hash = $1 AND active = true",
    )
    .bind(hash)
    .fetch_optional(pool)
    .await
}

/// Look up a publisher by its API key SHA-256 hash.
pub async fn get_publisher_by_hash(
    pool: &PgPool,
    hash: &str,
) -> Result<Option<Publisher>, sqlx::Error> {
    sqlx::query_as::<_, Publisher>(
        "SELECT * FROM publishers WHERE api_key_hash = $1 AND active = true",
    )
    .bind(hash)
    .fetch_optional(pool)
    .await
}

/// Load all active publishers (for domain index).
pub async fn list_active_publishers(pool: &PgPool) -> Result<Vec<Publisher>, sqlx::Error> {
    sqlx::query_as::<_, Publisher>("SELECT * FROM publishers WHERE active = true")
        .fetch_all(pool)
        .await
}

/// Get publisher summary with event counts filtered by their domains.
pub async fn get_publisher_summary(
    pool: &PgPool,
    publisher: &Publisher,
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
    domain_filter: Option<&str>,
) -> Result<PublisherSummary, sqlx::Error> {
    let domains = effective_domains(&publisher.domains, domain_filter);

    // Build LIKE patterns for domain matching
    let patterns: Vec<String> = domains
        .iter()
        .flat_map(|d| {
            vec![
                format!("https://{d}/%"),
                format!("https://www.{d}/%"),
                format!("http://{d}/%"),
                format!("http://www.{d}/%"),
            ]
        })
        .collect();

    if patterns.is_empty() {
        return Ok(empty_summary(publisher));
    }

    // Query event counts by type, filtered by domain patterns
    let mut query = String::from(
        "SELECT e.event_type, COUNT(*) as count, COUNT(DISTINCT e.session_id) as sessions
         FROM events e
         WHERE (",
    );

    // Build OR chain for LIKE patterns
    let like_clauses: Vec<String> = (1..=patterns.len())
        .map(|i| format!("e.content_url LIKE ${i}"))
        .collect();
    query.push_str(&like_clauses.join(" OR "));
    query.push(')');

    let mut param_idx = patterns.len() + 1;
    if since.is_some() {
        query.push_str(&format!(" AND e.event_timestamp >= ${param_idx}"));
        param_idx += 1;
    }
    if until.is_some() {
        query.push_str(&format!(" AND e.event_timestamp <= ${param_idx}"));
    }
    query.push_str(" GROUP BY e.event_type ORDER BY count DESC");

    let mut q = sqlx::query_as::<_, EventTypeCountRow>(&query);
    for pattern in &patterns {
        q = q.bind(pattern);
    }
    if let Some(ref s) = since {
        q = q.bind(s);
    }
    if let Some(ref u) = until {
        q = q.bind(u);
    }

    let rows = q.fetch_all(pool).await?;

    let total_events: i64 = rows.iter().map(|r| r.count).sum();
    let total_sessions: i64 = rows.iter().map(|r| r.sessions).max().unwrap_or(0);
    let events_by_type: Vec<EventTypeCount> = rows
        .into_iter()
        .map(|r| EventTypeCount {
            event_type: r.event_type,
            count: r.count,
        })
        .collect();

    // Agent breakdown: group events by platform_id/agent_id via session JOIN
    let agents = query_agent_breakdown(pool, &patterns, since, until).await?;

    Ok(PublisherSummary {
        publisher_id: publisher.id,
        publisher_name: publisher.name.clone(),
        domains: publisher.domains.clone(),
        total_events,
        total_sessions,
        events_by_type,
        agents,
        period_start: since,
        period_end: until,
    })
}

/// Get paginated events for a publisher's domains.
pub async fn get_publisher_events(
    pool: &PgPool,
    publisher: &Publisher,
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
    domain_filter: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Paginated<PublisherEvent>, sqlx::Error> {
    let domains = effective_domains(&publisher.domains, domain_filter);
    let patterns: Vec<String> = domain_like_patterns(&domains);

    if patterns.is_empty() {
        return Ok(Paginated {
            items: vec![],
            total: 0,
            limit,
            offset,
        });
    }

    // Count query
    let (count_sql, data_sql) = build_publisher_event_queries(&patterns, since, until);

    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    for p in &patterns {
        count_q = count_q.bind(p);
    }
    if let Some(ref s) = since {
        count_q = count_q.bind(s);
    }
    if let Some(ref u) = until {
        count_q = count_q.bind(u);
    }
    let total = count_q.fetch_one(pool).await?;

    // Data query with pagination
    let full_data_sql = format!("{data_sql} LIMIT ${} OFFSET ${}", patterns.len() + 1 + since.is_some() as usize + until.is_some() as usize, patterns.len() + 2 + since.is_some() as usize + until.is_some() as usize);
    let mut data_q = sqlx::query_as::<_, PublisherEventRow>(&full_data_sql);
    for p in &patterns {
        data_q = data_q.bind(p);
    }
    if let Some(ref s) = since {
        data_q = data_q.bind(s);
    }
    if let Some(ref u) = until {
        data_q = data_q.bind(u);
    }
    data_q = data_q.bind(limit).bind(offset);

    let rows = data_q.fetch_all(pool).await?;
    let items = rows
        .into_iter()
        .map(|r| PublisherEvent {
            event_id: r.id,
            session_id: r.session_id,
            event_type: r.event_type,
            content_url: r.content_url,
            event_timestamp: r.event_timestamp,
            event_data: r.event_data,
            platform_id: r.platform_id,
            agent_id: r.agent_id,
        })
        .collect();

    Ok(Paginated {
        items,
        total,
        limit,
        offset,
    })
}

/// Get URL-level metrics for a publisher's domains.
pub async fn get_publisher_url_metrics(
    pool: &PgPool,
    publisher: &Publisher,
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
    domain_filter: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Paginated<PublisherUrlMetric>, sqlx::Error> {
    let domains = effective_domains(&publisher.domains, domain_filter);
    let patterns = domain_like_patterns(&domains);

    if patterns.is_empty() {
        return Ok(Paginated {
            items: vec![],
            total: 0,
            limit,
            offset,
        });
    }

    // Build the WHERE clause for LIKE patterns
    let like_clauses: Vec<String> = (1..=patterns.len())
        .map(|i| format!("e.content_url LIKE ${i}"))
        .collect();
    let where_like = like_clauses.join(" OR ");

    let mut time_filter = String::new();
    let mut param_idx = patterns.len() + 1;
    if since.is_some() {
        time_filter.push_str(&format!(" AND e.event_timestamp >= ${param_idx}"));
        param_idx += 1;
    }
    if until.is_some() {
        time_filter.push_str(&format!(" AND e.event_timestamp <= ${param_idx}"));
        param_idx += 1;
    }

    // Count distinct URLs
    let count_sql = format!(
        "SELECT COUNT(DISTINCT e.content_url) FROM events e WHERE ({where_like}){time_filter} AND e.content_url IS NOT NULL"
    );
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    for p in &patterns {
        count_q = count_q.bind(p);
    }
    if let Some(ref s) = since {
        count_q = count_q.bind(s);
    }
    if let Some(ref u) = until {
        count_q = count_q.bind(u);
    }
    let total = count_q.fetch_one(pool).await?;

    // URL metrics query
    let data_sql = format!(
        "SELECT e.content_url, COUNT(*) as total_events,
                COUNT(DISTINCT e.session_id) as unique_sessions,
                MAX(e.event_timestamp) as last_seen
         FROM events e
         WHERE ({where_like}){time_filter} AND e.content_url IS NOT NULL
         GROUP BY e.content_url
         ORDER BY total_events DESC
         LIMIT ${param_idx} OFFSET ${}",
        param_idx + 1
    );

    let mut data_q = sqlx::query_as::<_, UrlMetricRow>(&data_sql);
    for p in &patterns {
        data_q = data_q.bind(p);
    }
    if let Some(ref s) = since {
        data_q = data_q.bind(s);
    }
    if let Some(ref u) = until {
        data_q = data_q.bind(u);
    }
    data_q = data_q.bind(limit).bind(offset);

    let rows = data_q.fetch_all(pool).await?;

    // For each URL, get event type breakdown
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        // Inline event type counts for this URL
        let type_counts = sqlx::query_as::<_, EventTypeCountRow>(
            "SELECT event_type, COUNT(*) as count, 0::bigint as sessions FROM events WHERE content_url = $1 GROUP BY event_type ORDER BY count DESC",
        )
        .bind(&row.content_url)
        .fetch_all(pool)
        .await?;

        items.push(PublisherUrlMetric {
            content_url: row.content_url,
            total_events: row.total_events,
            unique_sessions: row.unique_sessions,
            event_types: type_counts
                .into_iter()
                .map(|t| EventTypeCount {
                    event_type: t.event_type,
                    count: t.count,
                })
                .collect(),
            last_seen: row.last_seen,
        });
    }

    Ok(Paginated {
        items,
        total,
        limit,
        offset,
    })
}

/// Resolve a domain to a publisher.
pub fn resolve_domain_to_publisher<'a>(
    publishers: &'a [Publisher],
    domain: &str,
) -> Option<&'a Publisher> {
    let domain_lower = domain.to_lowercase();
    publishers.iter().find(|p| {
        p.domains.iter().any(|d| {
            let d = d.to_lowercase();
            domain_lower == d || domain_lower.ends_with(&format!(".{d}"))
        })
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn effective_domains<'a>(
    publisher_domains: &'a [String],
    domain_filter: Option<&str>,
) -> Vec<&'a str> {
    match domain_filter {
        Some(filter) => publisher_domains
            .iter()
            .filter(|d| {
                let d_lower = d.to_lowercase();
                let f_lower = filter.to_lowercase();
                d_lower == f_lower || d_lower.ends_with(&format!(".{f_lower}"))
            })
            .map(String::as_str)
            .collect(),
        None => publisher_domains.iter().map(String::as_str).collect(),
    }
}

fn domain_like_patterns(domains: &[&str]) -> Vec<String> {
    domains
        .iter()
        .flat_map(|d| {
            vec![
                format!("https://{d}/%"),
                format!("https://www.{d}/%"),
                format!("http://{d}/%"),
                format!("http://www.{d}/%"),
            ]
        })
        .collect()
}

fn build_publisher_event_queries(
    patterns: &[String],
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
) -> (String, String) {
    let like_clauses: Vec<String> = (1..=patterns.len())
        .map(|i| format!("e.content_url LIKE ${i}"))
        .collect();
    let where_like = like_clauses.join(" OR ");

    let mut time_filter = String::new();
    let mut param_idx = patterns.len() + 1;
    if since.is_some() {
        time_filter.push_str(&format!(" AND e.event_timestamp >= ${param_idx}"));
        param_idx += 1;
    }
    if until.is_some() {
        time_filter.push_str(&format!(" AND e.event_timestamp <= ${param_idx}"));
    }

    let count_sql = format!(
        "SELECT COUNT(*) FROM events e WHERE ({where_like}){time_filter}"
    );
    let data_sql = format!(
        "SELECT e.id, e.session_id, e.event_type, e.content_url, e.event_timestamp, e.event_data,
                s.platform_id, s.agent_id
         FROM events e
         JOIN sessions s ON e.session_id = s.id
         WHERE ({where_like}){time_filter}
         ORDER BY e.event_timestamp DESC"
    );

    (count_sql, data_sql)
}

async fn query_agent_breakdown(
    pool: &PgPool,
    patterns: &[String],
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
) -> Result<Vec<AgentBreakdown>, sqlx::Error> {
    let like_clauses: Vec<String> = (1..=patterns.len())
        .map(|i| format!("e.content_url LIKE ${i}"))
        .collect();
    let where_like = like_clauses.join(" OR ");

    let mut time_filter = String::new();
    let mut param_idx = patterns.len() + 1;
    if since.is_some() {
        time_filter.push_str(&format!(" AND e.event_timestamp >= ${param_idx}"));
        param_idx += 1;
    }
    if until.is_some() {
        time_filter.push_str(&format!(" AND e.event_timestamp <= ${param_idx}"));
    }
    let _ = param_idx;

    let sql = format!(
        "SELECT s.platform_id, s.agent_id,
                COUNT(*) as event_count,
                COUNT(DISTINCT e.session_id) as session_count
         FROM events e
         JOIN sessions s ON e.session_id = s.id
         WHERE ({where_like}){time_filter}
         GROUP BY s.platform_id, s.agent_id
         ORDER BY event_count DESC"
    );

    let mut q = sqlx::query_as::<_, AgentBreakdownRow>(&sql);
    for p in patterns {
        q = q.bind(p);
    }
    if let Some(ref s) = since {
        q = q.bind(s);
    }
    if let Some(ref u) = until {
        q = q.bind(u);
    }

    let rows = q.fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|r| AgentBreakdown {
            platform_id: r.platform_id,
            agent_id: r.agent_id,
            event_count: r.event_count,
            session_count: r.session_count,
        })
        .collect())
}

fn empty_summary(publisher: &Publisher) -> PublisherSummary {
    PublisherSummary {
        publisher_id: publisher.id,
        publisher_name: publisher.name.clone(),
        domains: publisher.domains.clone(),
        total_events: 0,
        total_sessions: 0,
        events_by_type: vec![],
        agents: vec![],
        period_start: None,
        period_end: None,
    }
}

// Internal row types for sqlx::FromRow
#[derive(Debug, sqlx::FromRow)]
struct EventTypeCountRow {
    event_type: String,
    count: i64,
    #[allow(dead_code)]
    sessions: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct PublisherEventRow {
    id: Uuid,
    session_id: Uuid,
    event_type: String,
    content_url: Option<String>,
    event_timestamp: DateTime<Utc>,
    event_data: serde_json::Value,
    platform_id: Option<String>,
    agent_id: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct AgentBreakdownRow {
    platform_id: Option<String>,
    agent_id: Option<String>,
    event_count: i64,
    session_count: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct UrlMetricRow {
    content_url: String,
    total_events: i64,
    unique_sessions: i64,
    last_seen: DateTime<Utc>,
}
