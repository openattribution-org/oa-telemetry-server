use std::sync::Arc;
use std::time::Duration;

use axum::extract::FromRef;
use dashmap::DashMap;
use moka::future::Cache;
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::ServerConfig;
use crate::models::publisher::{PlatformKey, Publisher};

/// Core state for the OpenAttribution Telemetry server.
///
/// Consumers embed this in their own state struct and implement `FromRef`.
#[derive(Clone)]
pub struct OaState {
    pub pool: PgPool,
    pub platform_cache: Cache<String, PlatformKey>,
    pub publisher_cache: Cache<String, Publisher>,
    pub session_cache: Cache<Uuid, bool>,
    pub domain_index: Arc<DashMap<String, Uuid>>,
    pub config: ServerConfig,
}

impl OaState {
    pub fn new(pool: PgPool, config: ServerConfig) -> Self {
        let auth_ttl = Duration::from_secs(config.auth_cache_ttl);

        Self {
            pool,
            platform_cache: Cache::builder()
                .max_capacity(10_000)
                .time_to_live(auth_ttl)
                .build(),
            publisher_cache: Cache::builder()
                .max_capacity(10_000)
                .time_to_live(auth_ttl)
                .build(),
            session_cache: Cache::builder()
                .max_capacity(config.session_cache_capacity)
                .time_to_live(Duration::from_secs(config.session_cache_ttl))
                .build(),
            domain_index: Arc::new(DashMap::new()),
            config,
        }
    }
}

impl FromRef<OaState> for PgPool {
    fn from_ref(state: &OaState) -> Self {
        state.pool.clone()
    }
}
