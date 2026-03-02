-- Click tokens: map click-out events to sessions for attribution lookup
-- When a user clicks through from an AI agent to a retailer/publisher landing page,
-- the URL carries a click token. The landing page can then query the OA server
-- to see which content was cited in the originating session.

CREATE TABLE IF NOT EXISTS click_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token       TEXT NOT NULL UNIQUE,
    session_id  UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    content_url TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '90 days')
);

CREATE INDEX idx_click_tokens_token ON click_tokens(token);
CREATE INDEX idx_click_tokens_session ON click_tokens(session_id);
CREATE INDEX idx_click_tokens_expires ON click_tokens(expires_at);
