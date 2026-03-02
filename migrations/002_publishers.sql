-- Publisher registry and platform API keys
-- Enables domain-based event routing and publisher dashboards

-- Publishers: content owners who receive telemetry about their URLs
CREATE TABLE IF NOT EXISTS publishers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    domains TEXT[] NOT NULL DEFAULT '{}',
    api_key_hash TEXT NOT NULL UNIQUE,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Platform API keys: for agent platforms that emit telemetry
CREATE TABLE IF NOT EXISTS platform_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    platform_id TEXT NOT NULL UNIQUE,
    api_key_hash TEXT NOT NULL UNIQUE,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_publishers_domains ON publishers USING GIN (domains);
CREATE INDEX idx_publishers_active ON publishers(active) WHERE active = true;
CREATE INDEX idx_platform_keys_active ON platform_keys(active) WHERE active = true;

CREATE TRIGGER publishers_updated_at
    BEFORE UPDATE ON publishers
    FOR EACH ROW
    EXECUTE FUNCTION oa_update_updated_at();

CREATE TRIGGER platform_keys_updated_at
    BEFORE UPDATE ON platform_keys
    FOR EACH ROW
    EXECUTE FUNCTION oa_update_updated_at();
