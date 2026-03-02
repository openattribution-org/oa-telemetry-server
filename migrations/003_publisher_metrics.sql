-- Pre-aggregated publisher metrics for dashboard queries
-- Populated by background aggregation (not real-time)

CREATE TABLE IF NOT EXISTS publisher_daily_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    publisher_id UUID NOT NULL REFERENCES publishers(id) ON DELETE CASCADE,
    metric_date DATE NOT NULL,
    domain TEXT NOT NULL,
    event_type TEXT NOT NULL,
    event_count BIGINT NOT NULL DEFAULT 0,
    unique_sessions BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT publisher_daily_metrics_unique UNIQUE (publisher_id, metric_date, domain, event_type)
);

CREATE INDEX idx_publisher_daily_metrics_lookup
    ON publisher_daily_metrics(publisher_id, metric_date DESC);

CREATE INDEX idx_publisher_daily_metrics_domain
    ON publisher_daily_metrics(publisher_id, domain, metric_date DESC);
