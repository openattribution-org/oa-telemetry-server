use axum::Router;
use axum::body::Body;
use http::StatusCode;
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use oa_telemetry_server::auth::hash_api_key;
use oa_telemetry_server::config::ServerConfig;
use oa_telemetry_server::state::OaState;

/// Build a test app with the full router.
pub fn build_test_app(pool: PgPool) -> Router {
    let config = ServerConfig {
        database_url: String::new(),
        port: 0,
        log_level: "error".to_string(),
        session_cache_capacity: 1000,
        session_cache_ttl: 60,
        auth_cache_ttl: 60,
        domain_refresh_secs: 3600,
    };

    let state = OaState::new(pool, config);
    oa_telemetry_server::router().with_state(state)
}

/// Build a test app with a pre-configured OaState.
pub fn build_test_app_with_state(state: OaState) -> Router {
    oa_telemetry_server::router().with_state(state)
}

/// Create a platform API key in the database. Returns the raw key.
pub async fn create_platform_key(pool: &PgPool, name: &str, platform_id: &str) -> String {
    let raw_key = format!("test-platform-key-{}", Uuid::new_v4());
    let hash = hash_api_key(&raw_key);

    sqlx::query(
        "INSERT INTO platform_keys (name, platform_id, api_key_hash) VALUES ($1, $2, $3)",
    )
    .bind(name)
    .bind(platform_id)
    .bind(hash)
    .execute(pool)
    .await
    .expect("Failed to create platform key");

    raw_key
}

/// Create a publisher in the database. Returns (publisher_id, raw_key).
pub async fn create_publisher(
    pool: &PgPool,
    name: &str,
    domains: &[&str],
) -> (Uuid, String) {
    let raw_key = format!("test-publisher-key-{}", Uuid::new_v4());
    let hash = hash_api_key(&raw_key);
    let domain_strings: Vec<String> = domains.iter().map(|d| d.to_string()).collect();

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO publishers (name, domains, api_key_hash) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(name)
    .bind(&domain_strings)
    .bind(hash)
    .fetch_one(pool)
    .await
    .expect("Failed to create publisher");

    (id, raw_key)
}

/// Helper to send a request and get the body as a serde_json::Value.
pub async fn send_json(
    app: Router,
    request: http::Request<Body>,
) -> (StatusCode, serde_json::Value) {
    let response = app
        .oneshot(request)
        .await
        .expect("Request failed");
    let status = response.status();
    let body_bytes = response
        .into_body()
        .collect()
        .await
        .expect("Body collect failed")
        .to_bytes();
    let value: serde_json::Value =
        serde_json::from_slice(&body_bytes).unwrap_or_else(|_| serde_json::json!(null));
    (status, value)
}
