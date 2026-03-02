# CLAUDE.md — oa-telemetry-server

## Project

OpenAttribution Telemetry Server — Rust implementation of the OA telemetry
processing server. Receives session/event telemetry from AI platforms, resolves
content URLs to publishers, and exposes publisher dashboards.

## Tech stack

- **Framework:** Axum 0.8 with Tokio async runtime
- **Database:** PostgreSQL via SQLx 0.8 (compile-time query checking disabled; runtime queries)
- **Config:** Figment (env vars), dotenvy for `.env` files
- **Auth:** SHA-256 API key hashing (platform keys + publisher keys)
- **Caching:** Moka (async TTL caches for auth/sessions), DashMap (domain index)
- **CLI:** Clap derive (`serve` / `keygen platform` / `keygen publisher`)
- **Container:** Podman/Docker via `Containerfile` (multi-stage, debian-slim runtime)
- **Edition:** Rust 2024

## Architecture

```
src/
  main.rs        — CLI entrypoint, server bootstrap, keygen commands
  lib.rs         — Public API: re-exports router, state, config, MIGRATOR
  config.rs      — ServerConfig (env-based via Figment)
  state.rs       — OaState: pool + caches + domain index (FromRef pattern)
  errors.rs      — OaError enum → Axum IntoResponse
  domain.rs      — URL→publisher resolution, domain hierarchy walking
  router.rs      — Route groups: write, publisher, internal, resolve, health
  auth/          — Platform + publisher API key auth extractors
  models/        — Session, Event, Publisher structs
  routes/        — Handler functions per route group
  services/      — Business logic (sessions, events, publishers, domain_index)
```

The router uses Axum's `FromRef` sub-state pattern so `OaState` can be embedded
in larger application state structs.

## Key patterns

- **Route groups** are separate functions (`write_routes()`, `publisher_routes()`, etc.)
  so consumers can `merge()` individual groups into their own routers.
- **Auth** is done via Axum extractors (`PlatformAuth`, `PublisherAuth`) — not middleware.
- **Domain index** is a `DashMap<String, Uuid>` refreshed periodically from the
  `publishers` table. Resolution walks up the domain hierarchy (e.g., `tech.ft.com` → `ft.com`).
- **Migrations** are embedded via `sqlx::migrate!("./migrations")` and exposed as `MIGRATOR`.

## Database

PostgreSQL with 3 migration files:
- `001_oa_telemetry.sql` — sessions + events tables
- `002_publishers.sql` — publishers + platform_keys tables
- `003_publisher_metrics.sql` — publisher_daily_metrics (pre-aggregated)

## Build & run

```sh
cargo build                          # dev build
cargo build --release                # release build
cargo run -- serve                   # start server (needs DATABASE_URL)
cargo run -- keygen platform --name "Test" --platform-id "test-agent"
cargo run -- keygen publisher --name "Test" --domains "example.com"
```

## Testing

Integration tests use `sqlx::PgPool` directly against a test database.
Test helpers are in `tests/common/mod.rs`.

```sh
cargo test                           # run all tests (needs DATABASE_URL)
cargo test --lib                     # unit tests only (no DB needed)
```

## Formatting & linting

```sh
cargo fmt --all                      # format (uses rustfmt.toml: edition = 2024)
cargo clippy --all-targets           # lint
```

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | *required* | PostgreSQL connection string |
| `PORT` | `8007` | Server listen port |
| `LOG_LEVEL` | `info` | Tracing filter level |
| `SESSION_CACHE_CAPACITY` | `50000` | Max cached session IDs |
| `SESSION_CACHE_TTL` | `3600` | Session cache TTL (seconds) |
| `AUTH_CACHE_TTL` | `300` | Auth lookup cache TTL (seconds) |
| `DOMAIN_REFRESH_SECS` | `300` | Domain index refresh interval |

## Conventions

- Use `thiserror` for error types, implement `IntoResponse` for HTTP mapping
- Prefer `tracing` macros over `println!`/`eprintln!` in library code
- Keep handlers thin — business logic belongs in `services/`
- SQL is written inline (no ORM) — use `sqlx::query` / `sqlx::query_as`
- Test via `tower::ServiceExt::oneshot` against the Axum router
