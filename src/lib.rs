//! OpenAttribution Telemetry Server
//!
//! Production-grade Rust implementation of the OpenAttribution Telemetry processing server.
//!
//! This crate exposes both a library (for embedding in larger applications like nai-telemetry)
//! and a standalone binary for independent deployment.
//!
//! # Architecture
//!
//! The server uses a generic router pattern via `FromRef` sub-state:
//!
//! ```rust,ignore
//! // Embed OaState in your own state struct
//! struct MyState {
//!     oa: OaState,
//!     // ... your other state
//! }
//!
//! impl FromRef<MyState> for OaState { ... }
//!
//! // Use individual route groups to avoid conflicts
//! let app = Router::new()
//!     .merge(oa_telemetry_server::internal_routes())
//!     .merge(oa_telemetry_server::resolve_routes())
//!     .with_state(my_state);
//! ```

pub mod auth;
pub mod config;
pub mod domain;
pub mod errors;
pub mod models;
pub mod router;
pub mod routes;
pub mod services;
pub mod state;

// Re-export key types for library consumers
pub use config::ServerConfig;
pub use errors::OaError;
pub use router::{
    health_routes, internal_routes, publisher_routes, resolve_routes, router, write_routes,
};
pub use state::OaState;

/// Embedded migrations for the OA telemetry schema.
///
/// Consumers run these before their own extensions:
/// ```rust,ignore
/// oa_telemetry_server::MIGRATOR.run(&pool).await?;
/// ```
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
