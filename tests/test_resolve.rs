mod common;

use axum::body::Body;
use http::StatusCode;
use sqlx::PgPool;

use oa_telemetry_server::config::ServerConfig;
use oa_telemetry_server::services::domain_index;
use oa_telemetry_server::state::OaState;

use common::{build_test_app_with_state, create_publisher, send_json};

#[sqlx::test(migrations = "./migrations")]
async fn test_resolve_known_domain(pool: PgPool) {
    // GIVEN a publisher with domains and a populated domain index
    let (_pub_id, _pub_key) = create_publisher(&pool, "BBC", &["bbc.co.uk"]).await;

    let config = ServerConfig {
        database_url: String::new(),
        port: 0,
        log_level: "error".to_string(),
        session_cache_capacity: 1000,
        session_cache_ttl: 60,
        auth_cache_ttl: 60,
        domain_refresh_secs: 3600,
    };
    let state = OaState::new(pool.clone(), config);
    domain_index::refresh_domain_index(&pool, &state.domain_index)
        .await
        .unwrap();

    let app = build_test_app_with_state(state);

    // WHEN resolving a URL for that domain
    let request = http::Request::builder()
        .method("GET")
        .uri("/resolve?url=https%3A%2F%2Fwww.bbc.co.uk%2Fnews%2Farticle")
        .body(Body::empty())
        .unwrap();

    let (status, body) = send_json(app, request).await;

    // SHOULD return handled=true with publisher info
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["handled"], true);
    assert_eq!(body["publisher"]["name"], "BBC");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_resolve_unknown_domain(pool: PgPool) {
    // GIVEN no publishers registered
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
    let app = build_test_app_with_state(state);

    // WHEN resolving an unknown domain
    let request = http::Request::builder()
        .method("GET")
        .uri("/resolve?domain=unknown-domain.com")
        .body(Body::empty())
        .unwrap();

    let (status, body) = send_json(app, request).await;

    // SHOULD return handled=false
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["handled"], false);
    assert!(body["publisher"].is_null());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_well_known_endpoint(pool: PgPool) {
    // GIVEN a publisher and populated domain index
    let (_pub_id, _pub_key) = create_publisher(&pool, "FT", &["ft.com"]).await;

    let config = ServerConfig {
        database_url: String::new(),
        port: 0,
        log_level: "error".to_string(),
        session_cache_capacity: 1000,
        session_cache_ttl: 60,
        auth_cache_ttl: 60,
        domain_refresh_secs: 3600,
    };
    let state = OaState::new(pool.clone(), config);
    domain_index::refresh_domain_index(&pool, &state.domain_index)
        .await
        .unwrap();

    let app = build_test_app_with_state(state);

    // WHEN fetching .well-known
    let request = http::Request::builder()
        .method("GET")
        .uri("/.well-known/openattribution.json")
        .body(Body::empty())
        .unwrap();

    let (status, body) = send_json(app, request).await;

    // SHOULD return server metadata with registered domains
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["openattribution"]["version"], "0.4");
    assert_eq!(body["openattribution"]["server"], "oa-telemetry-server");

    let domains = body["openattribution"]["registered_domains"]
        .as_array()
        .unwrap();
    assert!(domains.iter().any(|d| d == "ft.com"));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_resolve_requires_url_or_domain_param(pool: PgPool) {
    // GIVEN no params
    let app = common::build_test_app(pool);

    let request = http::Request::builder()
        .method("GET")
        .uri("/resolve")
        .body(Body::empty())
        .unwrap();

    let (status, _body) = send_json(app, request).await;

    // SHOULD return 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
