use sqlx::PgPool;
use uuid::Uuid;

use crate::models::event::{EventRow, TelemetryEventInput};

/// Batch-insert events using UNNEST for single round-trip performance.
pub async fn create_events(
    pool: &PgPool,
    session_id: Uuid,
    events: &[TelemetryEventInput],
) -> Result<Vec<EventRow>, sqlx::Error> {
    if events.is_empty() {
        return Ok(Vec::new());
    }

    let len = events.len();
    let mut ids = Vec::with_capacity(len);
    let mut session_ids = Vec::with_capacity(len);
    let mut event_types = Vec::with_capacity(len);
    let mut content_urls: Vec<Option<String>> = Vec::with_capacity(len);
    let mut product_ids: Vec<Option<Uuid>> = Vec::with_capacity(len);
    let mut turn_datas: Vec<Option<serde_json::Value>> = Vec::with_capacity(len);
    let mut event_datas = Vec::with_capacity(len);
    let mut timestamps = Vec::with_capacity(len);

    for event in events {
        ids.push(event.id);
        session_ids.push(session_id);
        event_types.push(event.event_type.clone());
        content_urls.push(event.content_url.clone());
        product_ids.push(event.product_id);
        turn_datas.push(event.turn.clone());
        event_datas.push(event.data.clone());
        timestamps.push(event.timestamp);
    }

    sqlx::query_as::<_, EventRow>(
        r"INSERT INTO events (
            id, session_id, event_type, content_url, product_id,
            turn_data, event_data, event_timestamp
        )
        SELECT * FROM UNNEST(
            $1::uuid[], $2::uuid[], $3::text[], $4::text[], $5::uuid[],
            $6::jsonb[], $7::jsonb[], $8::timestamptz[]
        )
        RETURNING *",
    )
    .bind(&ids)
    .bind(&session_ids)
    .bind(&event_types)
    .bind(&content_urls)
    .bind(&product_ids)
    .bind(&turn_datas)
    .bind(&event_datas)
    .bind(&timestamps)
    .fetch_all(pool)
    .await
}

/// Get all events for a session, ordered by timestamp.
pub async fn get_events_for_session(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Vec<EventRow>, sqlx::Error> {
    sqlx::query_as::<_, EventRow>(
        "SELECT * FROM events WHERE session_id = $1 ORDER BY event_timestamp ASC",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
}
