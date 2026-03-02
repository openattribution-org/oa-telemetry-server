-- OpenAttribution Telemetry Server Schema
-- Production Rust implementation matching the reference server (v0.4)
-- https://github.com/openattribution-org/telemetry

-- Helper function for updated_at timestamps
CREATE OR REPLACE FUNCTION oa_update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Sessions table
-- Represents a bounded interaction between an initiator (user or agent) and an AI agent
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Actor types: who initiated this session
    initiator_type TEXT NOT NULL DEFAULT 'user',

    -- Initiator identity (for agent-to-agent sessions)
    initiator JSONB,

    -- Content scope: opaque identifier for the content collection/permissions context
    content_scope TEXT,

    -- AIMS manifest reference for licensing verification
    manifest_ref TEXT,

    -- Fraud prevention: hash of content configuration at session start
    config_snapshot_hash TEXT,

    -- Agent identifier (for multi-agent systems)
    agent_id TEXT,

    -- External session ID for lookups
    external_session_id TEXT,

    -- Cross-session journey linking
    prior_session_ids UUID[] DEFAULT '{}',

    -- User context for segmentation (no PII)
    user_context JSONB NOT NULL DEFAULT '{}',

    -- SPUR extensions: platform identification
    platform_id TEXT,
    client_type TEXT,
    client_info JSONB,

    -- Session lifecycle
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ,

    -- Session outcome
    outcome_type TEXT,
    outcome_value JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT sessions_initiator_type_check CHECK (
        initiator_type IN ('user', 'agent')
    ),

    CONSTRAINT sessions_outcome_type_check CHECK (
        outcome_type IS NULL OR outcome_type IN ('conversion', 'abandonment', 'browse')
    )
);

-- Events table
-- Individual telemetry events within a session
CREATE TABLE IF NOT EXISTS events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,

    event_type TEXT NOT NULL,
    content_url TEXT,
    product_id UUID,
    turn_data JSONB,
    event_data JSONB NOT NULL DEFAULT '{}',
    event_timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT events_type_check CHECK (
        event_type IN (
            'content_retrieved', 'content_displayed', 'content_engaged', 'content_cited',
            'turn_started', 'turn_completed',
            'product_viewed', 'product_compared', 'cart_add', 'cart_remove',
            'checkout_started', 'checkout_completed', 'checkout_abandoned'
        )
    )
);

-- Indexes
CREATE INDEX idx_sessions_scope ON sessions(content_scope) WHERE content_scope IS NOT NULL;
CREATE INDEX idx_sessions_external ON sessions(external_session_id) WHERE external_session_id IS NOT NULL;
CREATE INDEX idx_sessions_outcome ON sessions(outcome_type) WHERE outcome_type IS NOT NULL;
CREATE INDEX idx_sessions_ended ON sessions(ended_at) WHERE ended_at IS NOT NULL;
CREATE INDEX idx_sessions_platform ON sessions(platform_id) WHERE platform_id IS NOT NULL;
CREATE INDEX idx_events_session ON events(session_id, event_timestamp);
CREATE INDEX idx_events_content ON events(content_url) WHERE content_url IS NOT NULL;
CREATE INDEX idx_sessions_prior ON sessions USING GIN (prior_session_ids);

-- Auto-update updated_at timestamp
CREATE TRIGGER sessions_updated_at
    BEFORE UPDATE ON sessions
    FOR EACH ROW
    EXECUTE FUNCTION oa_update_updated_at();
