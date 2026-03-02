use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::click_token::{ClickTokenRow, SessionContextResponse};

/// Create a click token mapping a click-out event to a session.
///
/// If `token` is None, a random UUID-based token is generated.
pub async fn create_click_token(
    pool: &PgPool,
    session_id: Uuid,
    content_url: &str,
    token: Option<&str>,
) -> Result<ClickTokenRow, sqlx::Error> {
    let token_value = token
        .map(String::from)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    sqlx::query_as::<_, ClickTokenRow>(
        "INSERT INTO click_tokens (token, session_id, content_url)
         VALUES ($1, $2, $3)
         RETURNING *",
    )
    .bind(&token_value)
    .bind(session_id)
    .bind(content_url)
    .fetch_one(pool)
    .await
}

/// Look up session context by click token.
///
/// Returns the cited and retrieved content URLs from the session,
/// along with the click-out URL and session metadata.
/// Returns None if the token doesn't exist or has expired.
pub async fn lookup_by_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<SessionContextResponse>, sqlx::Error> {
    // Fetch the click token row (check expiry)
    let click_token = sqlx::query_as::<_, ClickTokenRow>(
        "SELECT * FROM click_tokens WHERE token = $1 AND expires_at > NOW()",
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    let click_token = match click_token {
        Some(ct) => ct,
        None => return Ok(None),
    };

    // Get session start time
    let started_at: DateTime<Utc> =
        sqlx::query_scalar("SELECT started_at FROM sessions WHERE id = $1")
            .bind(click_token.session_id)
            .fetch_one(pool)
            .await?;

    // Get cited content URLs
    let cited: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT content_url FROM events
         WHERE session_id = $1 AND event_type = 'content_cited' AND content_url IS NOT NULL",
    )
    .bind(click_token.session_id)
    .fetch_all(pool)
    .await?;

    // Get retrieved content URLs
    let retrieved: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT content_url FROM events
         WHERE session_id = $1 AND event_type = 'content_retrieved' AND content_url IS NOT NULL",
    )
    .bind(click_token.session_id)
    .fetch_all(pool)
    .await?;

    Ok(Some(SessionContextResponse {
        session_id: click_token.session_id,
        started_at,
        click_content_url: click_token.content_url,
        content_urls_cited: cited,
        content_urls_retrieved: retrieved,
    }))
}

/// Delete expired click tokens.
pub async fn cleanup_expired(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM click_tokens WHERE expires_at < NOW()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}
