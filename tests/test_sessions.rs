mod common;

use axum::body::Body;
use http::StatusCode;
use sqlx::PgPool;

use common::{build_test_app, create_platform_key, send_json};

#[sqlx::test(migrations = "./migrations")]
async fn test_start_session_returns_session_id(pool: PgPool) {
    // GIVEN a platform key and the test app
    let api_key = create_platform_key(&pool, "Test Platform", "test-platform-1").await;
    let app = build_test_app(pool);

    // WHEN starting a session
    let request = http::Request::builder()
        .method("POST")
        .uri("/session/start")
        .header("Content-Type", "application/json")
        .header("X-API-Key", &api_key)
        .body(Body::from(
            r#"{"initiator_type": "user", "content_scope": "test-scope"}"#,
        ))
        .unwrap();

    let (status, body) = send_json(app, request).await;

    // SHOULD return 201 with a valid session_id
    assert_eq!(status, StatusCode::CREATED);
    assert!(body["session_id"].is_string());
    let session_id = body["session_id"].as_str().unwrap();
    assert!(uuid::Uuid::parse_str(session_id).is_ok());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_start_session_without_api_key_returns_401(pool: PgPool) {
    // GIVEN no API key
    let app = build_test_app(pool);

    // WHEN starting a session without auth
    let request = http::Request::builder()
        .method("POST")
        .uri("/session/start")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"initiator_type": "user"}"#))
        .unwrap();

    let (status, _body) = send_json(app, request).await;

    // SHOULD return 401
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_end_session_records_outcome(pool: PgPool) {
    // GIVEN a started session
    let api_key = create_platform_key(&pool, "Test Platform", "test-platform-2").await;
    let app = build_test_app(pool);

    let start_req = http::Request::builder()
        .method("POST")
        .uri("/session/start")
        .header("Content-Type", "application/json")
        .header("X-API-Key", &api_key)
        .body(Body::from(r#"{"initiator_type": "user"}"#))
        .unwrap();

    let (_, start_body) = send_json(app.clone(), start_req).await;
    let session_id = start_body["session_id"].as_str().unwrap().to_string();

    // WHEN ending the session with a conversion outcome
    let end_req = http::Request::builder()
        .method("POST")
        .uri("/session/end")
        .header("Content-Type", "application/json")
        .header("X-API-Key", &api_key)
        .body(Body::from(format!(
            r#"{{"session_id": "{session_id}", "outcome": {{"type": "conversion", "value_amount": 4999, "currency": "GBP"}}}}"#
        )))
        .unwrap();

    let (status, body) = send_json(app, end_req).await;

    // SHOULD return 200 with ok status
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["session_id"], session_id);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_bulk_session_creates_session_events_outcome(pool: PgPool) {
    // GIVEN a platform key
    let api_key = create_platform_key(&pool, "Test Platform", "test-platform-3").await;
    let app = build_test_app(pool);

    // WHEN uploading a complete session via bulk
    let request = http::Request::builder()
        .method("POST")
        .uri("/session/bulk")
        .header("Content-Type", "application/json")
        .header("X-API-Key", &api_key)
        .body(Body::from(
            r#"{
                "session_id": "00000000-0000-0000-0000-000000000001",
                "initiator_type": "user",
                "content_scope": "bulk-test",
                "events": [
                    {
                        "id": "00000000-0000-0000-0000-000000000010",
                        "type": "content_retrieved",
                        "timestamp": "2026-01-15T10:00:00Z",
                        "content_url": "https://example.com/article-1"
                    },
                    {
                        "id": "00000000-0000-0000-0000-000000000011",
                        "type": "content_cited",
                        "timestamp": "2026-01-15T10:00:01Z",
                        "content_url": "https://example.com/article-1",
                        "data": {"citation_type": "direct_quote"}
                    }
                ],
                "outcome": {
                    "type": "conversion",
                    "value_amount": 2999,
                    "currency": "USD"
                }
            }"#,
        ))
        .unwrap();

    let (status, body) = send_json(app, request).await;

    // SHOULD return 201 with session_id, events_created, outcome_recorded
    assert_eq!(status, StatusCode::CREATED);
    assert!(body["session_id"].is_string());
    assert_eq!(body["events_created"], 2);
    assert_eq!(body["outcome_recorded"], true);
}
