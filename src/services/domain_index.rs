use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use sqlx::PgPool;
use uuid::Uuid;

use crate::services::publishers;

/// Preload domain -> publisher_id mapping from the database.
pub async fn load_domain_index(
    pool: &PgPool,
) -> Result<Arc<DashMap<String, Uuid>>, sqlx::Error> {
    let index = Arc::new(DashMap::new());
    refresh_domain_index(pool, &index).await?;
    Ok(index)
}

/// Refresh the domain index from the database.
pub async fn refresh_domain_index(
    pool: &PgPool,
    index: &DashMap<String, Uuid>,
) -> Result<(), sqlx::Error> {
    let active_publishers = publishers::list_active_publishers(pool).await?;

    index.clear();
    for publisher in &active_publishers {
        for domain in &publisher.domains {
            let normalised = domain
                .strip_prefix("www.")
                .unwrap_or(domain)
                .to_lowercase();
            index.insert(normalised, publisher.id);
        }
    }

    tracing::info!(
        publisher_count = active_publishers.len(),
        domain_count = index.len(),
        "Domain index refreshed"
    );

    Ok(())
}

/// Spawn a background task that refreshes the domain index periodically.
pub fn spawn_refresh_task(
    pool: PgPool,
    index: Arc<DashMap<String, Uuid>>,
    interval_secs: u64,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            if let Err(e) = refresh_domain_index(&pool, &index).await {
                tracing::warn!(error = %e, "Failed to refresh domain index");
            }
        }
    });
}
