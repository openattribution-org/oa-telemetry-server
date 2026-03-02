mod common;

use axum::body::Body;
use http::StatusCode;
use sqlx::PgPool;

use common::{build_test_app, create_platform_key, send_json};

#[sqlx::test(migrations = "./migrations")]
async fn test_record_events_for_active_session(pool: PgPool) {
    // GIVEN an active session
    let api_key = create_platform_key(&pool, "Test Platform", "test-events-1").await;
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

    // WHEN recording events
    let events_req = http::Request::builder()
        .method("POST")
        .uri("/events")
        .header("Content-Type", "application/json")
        .header("X-API-Key", &api_key)
        .body(Body::from(format!(
            r#"{{
                "session_id": "{session_id}",
                "events": [
                    {{
                        "id": "00000000-0000-0000-0000-000000000020",
                        "type": "content_retrieved",
                        "timestamp": "2026-01-15T10:00:00Z",
                        "content_url": "https://bbc.co.uk/news/article"
                    }},
                    {{
                        "id": "00000000-0000-0000-0000-000000000021",
                        "type": "product_viewed",
                        "timestamp": "2026-01-15T10:00:01Z",
                        "product_id": "00000000-0000-0000-0000-000000000099"
                    }}
                ]
            }}"#
        )))
        .unwrap();

    let (status, body) = send_json(app, events_req).await;

    // SHOULD return 201 with events_created count
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["events_created"], 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_record_events_for_ended_session_fails(pool: PgPool) {
    // GIVEN a session that has been ended
    let api_key = create_platform_key(&pool, "Test Platform", "test-events-2").await;
    let app = build_test_app(pool);

    // Start session
    let start_req = http::Request::builder()
        .method("POST")
        .uri("/session/start")
        .header("Content-Type", "application/json")
        .header("X-API-Key", &api_key)
        .body(Body::from(r#"{"initiator_type": "user"}"#))
        .unwrap();

    let (_, start_body) = send_json(app.clone(), start_req).await;
    let session_id = start_body["session_id"].as_str().unwrap().to_string();

    // End session
    let end_req = http::Request::builder()
        .method("POST")
        .uri("/session/end")
        .header("Content-Type", "application/json")
        .header("X-API-Key", &api_key)
        .body(Body::from(format!(
            r#"{{"session_id": "{session_id}", "outcome": {{"type": "browse"}}}}"#
        )))
        .unwrap();
    send_json(app.clone(), end_req).await;

    // WHEN trying to add events to the ended session
    let events_req = http::Request::builder()
        .method("POST")
        .uri("/events")
        .header("Content-Type", "application/json")
        .header("X-API-Key", &api_key)
        .body(Body::from(format!(
            r#"{{
                "session_id": "{session_id}",
                "events": [{{
                    "id": "00000000-0000-0000-0000-000000000030",
                    "type": "content_retrieved",
                    "timestamp": "2026-01-15T10:00:00Z"
                }}]
            }}"#
        )))
        .unwrap();

    let (status, _body) = send_json(app, events_req).await;

    // SHOULD return 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_record_events_for_nonexistent_session_fails(pool: PgPool) {
    // GIVEN a non-existent session ID
    let api_key = create_platform_key(&pool, "Test Platform", "test-events-3").await;
    let app = build_test_app(pool);

    let events_req = http::Request::builder()
        .method("POST")
        .uri("/events")
        .header("Content-Type", "application/json")
        .header("X-API-Key", &api_key)
        .body(Body::from(
            r#"{
                "session_id": "00000000-0000-0000-0000-999999999999",
                "events": [{
                    "id": "00000000-0000-0000-0000-000000000040",
                    "type": "content_retrieved",
                    "timestamp": "2026-01-15T10:00:00Z"
                }]
            }"#,
        ))
        .unwrap();

    let (status, _body) = send_json(app, events_req).await;

    // SHOULD return 404
    assert_eq!(status, StatusCode::NOT_FOUND);
}
