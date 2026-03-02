mod common;

use axum::body::Body;
use http::StatusCode;
use sqlx::PgPool;

use common::{build_test_app, create_platform_key, create_publisher, send_json};

/// Helper: create a session with events containing URLs for a given domain.
async fn seed_events(
    app: axum::Router,
    api_key: &str,
    domain: &str,
) -> String {
    // Start session
    let start_req = http::Request::builder()
        .method("POST")
        .uri("/session/start")
        .header("Content-Type", "application/json")
        .header("X-API-Key", api_key)
        .body(Body::from(r#"{"initiator_type": "user"}"#))
        .unwrap();

    let (_, start_body) = send_json(app.clone(), start_req).await;
    let session_id = start_body["session_id"].as_str().unwrap().to_string();

    // Record events with URLs for the domain
    let events_body = format!(
        r#"{{
            "session_id": "{session_id}",
            "events": [
                {{
                    "id": "00000000-0000-0000-0000-{:012x}",
                    "type": "content_retrieved",
                    "timestamp": "2026-01-15T10:00:00Z",
                    "content_url": "https://{domain}/article-1"
                }},
                {{
                    "id": "00000000-0000-0000-0000-{:012x}",
                    "type": "content_cited",
                    "timestamp": "2026-01-15T10:00:01Z",
                    "content_url": "https://{domain}/article-1"
                }}
            ]
        }}"#,
        rand::random_range(0..0xFFFF_FFFF_FFFFu64),
        rand::random_range(0..0xFFFF_FFFF_FFFFu64),
    );

    let events_req = http::Request::builder()
        .method("POST")
        .uri("/events")
        .header("Content-Type", "application/json")
        .header("X-API-Key", api_key)
        .body(Body::from(events_body))
        .unwrap();

    send_json(app, events_req).await;
    session_id
}

#[sqlx::test(migrations = "./migrations")]
async fn test_publisher_summary_returns_event_counts(pool: PgPool) {
    // GIVEN a publisher with domains and some events for those domains
    let platform_key = create_platform_key(&pool, "Platform", "pub-query-1").await;
    let (_pub_id, pub_key) = create_publisher(&pool, "BBC", &["bbc.co.uk"]).await;
    let app = build_test_app(pool);

    seed_events(app.clone(), &platform_key, "bbc.co.uk").await;

    // WHEN querying publisher summary
    let request = http::Request::builder()
        .method("GET")
        .uri("/publisher/summary")
        .header("X-API-Key", &pub_key)
        .body(Body::empty())
        .unwrap();

    let (status, body) = send_json(app, request).await;

    // SHOULD return summary with event counts
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["publisher_name"], "BBC");
    assert!(body["total_events"].as_i64().unwrap() >= 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_publisher_events_filtered_by_domain(pool: PgPool) {
    // GIVEN events for two different domains, one publisher owns only one
    let platform_key = create_platform_key(&pool, "Platform", "pub-query-2").await;
    let (_bbc_id, bbc_key) = create_publisher(&pool, "BBC", &["bbc.co.uk"]).await;
    let (_ft_id, _ft_key) = create_publisher(&pool, "FT", &["ft.com"]).await;
    let app = build_test_app(pool);

    seed_events(app.clone(), &platform_key, "bbc.co.uk").await;
    seed_events(app.clone(), &platform_key, "ft.com").await;

    // WHEN BBC queries their events
    let request = http::Request::builder()
        .method("GET")
        .uri("/publisher/events?limit=100")
        .header("X-API-Key", &bbc_key)
        .body(Body::empty())
        .unwrap();

    let (status, body) = send_json(app, request).await;

    // SHOULD only see BBC events, not FT events
    assert_eq!(status, StatusCode::OK);
    let items = body["items"].as_array().unwrap();
    for item in items {
        let url = item["content_url"].as_str().unwrap_or("");
        assert!(
            url.contains("bbc.co.uk"),
            "Expected BBC URLs only, got: {url}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn test_publisher_urls_returns_url_metrics(pool: PgPool) {
    // GIVEN events for a publisher's domain
    let platform_key = create_platform_key(&pool, "Platform", "pub-query-3").await;
    let (_pub_id, pub_key) = create_publisher(&pool, "Guardian", &["theguardian.com"]).await;
    let app = build_test_app(pool);

    seed_events(app.clone(), &platform_key, "theguardian.com").await;

    // WHEN querying URL metrics
    let request = http::Request::builder()
        .method("GET")
        .uri("/publisher/urls?limit=10")
        .header("X-API-Key", &pub_key)
        .body(Body::empty())
        .unwrap();

    let (status, body) = send_json(app, request).await;

    // SHOULD return URL-level metrics
    assert_eq!(status, StatusCode::OK);
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    assert!(items[0]["content_url"].is_string());
    assert!(items[0]["total_events"].as_i64().unwrap() > 0);
}
