mod common;

use axum::body::Body;
use http::StatusCode;
use sqlx::PgPool;

use common::{build_test_app, create_platform_key, send_json};

#[sqlx::test(migrations = "./migrations")]
async fn test_invalid_api_key_returns_401(pool: PgPool) {
    // GIVEN an invalid API key
    let app = build_test_app(pool);

    let request = http::Request::builder()
        .method("POST")
        .uri("/session/start")
        .header("Content-Type", "application/json")
        .header("X-API-Key", "definitely-not-a-valid-key")
        .body(Body::from(r#"{"initiator_type": "user"}"#))
        .unwrap();

    let (status, body) = send_json(app, request).await;

    // SHOULD return 401
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"].as_str().unwrap().contains("Invalid"));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_valid_platform_key_succeeds(pool: PgPool) {
    // GIVEN a valid platform key
    let api_key = create_platform_key(&pool, "Valid Platform", "valid-platform").await;
    let app = build_test_app(pool);

    let request = http::Request::builder()
        .method("POST")
        .uri("/session/start")
        .header("Content-Type", "application/json")
        .header("X-API-Key", &api_key)
        .body(Body::from(r#"{"initiator_type": "user"}"#))
        .unwrap();

    let (status, _body) = send_json(app, request).await;

    // SHOULD succeed
    assert_eq!(status, StatusCode::CREATED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_publisher_key_does_not_work_for_write_endpoints(pool: PgPool) {
    // GIVEN a publisher key (not a platform key)
    let (_pub_id, pub_key) =
        common::create_publisher(&pool, "Test Publisher", &["example.com"]).await;
    let app = build_test_app(pool);

    // WHEN using a publisher key on a write endpoint
    let request = http::Request::builder()
        .method("POST")
        .uri("/session/start")
        .header("Content-Type", "application/json")
        .header("X-API-Key", &pub_key)
        .body(Body::from(r#"{"initiator_type": "user"}"#))
        .unwrap();

    let (status, _body) = send_json(app, request).await;

    // SHOULD return 401 (publisher keys are not platform keys)
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_internal_endpoints_require_no_auth(pool: PgPool) {
    // GIVEN no API key
    let app = build_test_app(pool);

    // WHEN accessing internal endpoints
    let request = http::Request::builder()
        .method("GET")
        .uri("/internal/sessions?limit=1")
        .body(Body::empty())
        .unwrap();

    let (status, _body) = send_json(app, request).await;

    // SHOULD succeed without auth
    assert_eq!(status, StatusCode::OK);
}
