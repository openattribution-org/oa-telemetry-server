# oa-telemetry-server

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

Production Rust implementation of the [OpenAttribution](https://openattribution.org) telemetry server. Receives session and event telemetry from AI platforms, resolves content URLs to publishers, and serves publisher dashboards.

Compatible with the OpenAttribution [Python SDK](https://pypi.org/project/openattribution-telemetry/) and [TypeScript SDK](https://www.npmjs.com/package/@openattribution/telemetry).

## Quick start

Requires PostgreSQL and Rust 1.85+ (edition 2024).

```bash
# Set up the database
export DATABASE_URL="postgres://user:pass@localhost/oa_telemetry"

# Build and run
cargo run -- serve
```

The server runs migrations automatically on startup.

## Generate API keys

Platforms (AI agents emitting telemetry) and publishers (content owners reading telemetry) authenticate with API keys:

```bash
# Platform key — for agents sending telemetry
cargo run -- keygen platform --name "My Agent" --platform-id "my-agent"

# Publisher key — for content owners querying their data
cargo run -- keygen publisher --name "Example News" --domains "example.com,news.example.com"
```

Keys are displayed once. The server stores only the SHA-256 hash.

## API

All write endpoints require a platform API key via the `X-API-Key` header.
Publisher endpoints require a publisher API key via the same header.

### Write (platform auth)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/session/start` | Start a new telemetry session |
| `POST` | `/session/end` | End a session with an outcome |
| `POST` | `/session/bulk` | Upload a complete session in one request |
| `POST` | `/events` | Record events for an active session |

### Publisher queries (publisher auth)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/publisher/summary` | Aggregate metrics for the publisher's domains |
| `GET` | `/publisher/events` | Paginated event list for the publisher's domains |
| `GET` | `/publisher/urls` | Per-URL metrics for the publisher's domains |

Query params: `since`, `until`, `domain`, `limit`, `offset`

### Internal (no auth)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/internal/sessions/{id}` | Get session with events |
| `GET` | `/internal/sessions` | List sessions (filterable) |
| `GET` | `/internal/sessions/by-external-id/{id}` | Look up by external ID |

### Discovery (no auth)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/resolve?url=...` | Resolve a URL to its registered publisher |
| `GET` | `/.well-known/openattribution.json` | Server metadata and registered domains |

### Health

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Always returns `{"status": "ok"}` |
| `GET` | `/ready` | Checks database connectivity |

## Configuration

All configuration is via environment variables. A `.env` file is loaded automatically.

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | *required* | PostgreSQL connection string |
| `PORT` | `8007` | Listen port |
| `LOG_LEVEL` | `info` | Tracing filter (`debug`, `info`, `warn`, `error`) |
| `SESSION_CACHE_CAPACITY` | `50000` | Max session IDs held in cache |
| `SESSION_CACHE_TTL` | `3600` | Session cache TTL in seconds |
| `AUTH_CACHE_TTL` | `300` | Auth lookup cache TTL in seconds |
| `DOMAIN_REFRESH_SECS` | `300` | How often to reload the domain index from the DB |

## Embedding as a library

The server is designed to be embedded in larger applications. Use individual route groups to avoid conflicts with your own routes:

```rust
use axum::extract::FromRef;
use oa_telemetry_server::OaState;

#[derive(Clone)]
struct MyState {
    oa: OaState,
    // ... your other state
}

impl FromRef<MyState> for OaState {
    fn from_ref(state: &MyState) -> Self {
        state.oa.clone()
    }
}

let app = axum::Router::new()
    .merge(oa_telemetry_server::write_routes())
    .merge(oa_telemetry_server::resolve_routes())
    .merge(oa_telemetry_server::health_routes())
    // add your own routes ...
    .with_state(my_state);

// Run embedded migrations before starting
oa_telemetry_server::MIGRATOR.run(&pool).await?;
```

## Container

```bash
podman build -f Containerfile -t oa-telemetry-server .
podman run -e DATABASE_URL="postgres://..." -p 8007:8007 oa-telemetry-server
```

## Development

```bash
cargo build              # dev build
cargo test --lib         # unit tests (no DB)
cargo test               # all tests (needs DATABASE_URL)
cargo fmt --all          # format
cargo clippy             # lint
```

## License

Apache 2.0
