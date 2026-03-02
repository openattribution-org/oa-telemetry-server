use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use oa_telemetry_server::auth::{generate_raw_key, hash_api_key};
use oa_telemetry_server::config::ServerConfig;
use oa_telemetry_server::services::domain_index;
use oa_telemetry_server::state::OaState;

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "oa-telemetry-server", version, about = "OpenAttribution Telemetry Server")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start the telemetry server (default when no subcommand given)
    Serve,
    /// Generate API keys for platforms or publishers
    Keygen {
        #[command(subcommand)]
        target: KeygenTarget,
    },
}

#[derive(Subcommand)]
enum KeygenTarget {
    /// Generate a platform API key (for AI platforms emitting telemetry)
    Platform {
        /// Platform display name (e.g. "Forage")
        #[arg(long)]
        name: String,
        /// Unique platform identifier (e.g. "forage-agent")
        #[arg(long)]
        platform_id: String,
    },
    /// Generate a publisher API key (for publishers reading their telemetry)
    Publisher {
        /// Publisher display name (e.g. "The Guardian")
        #[arg(long)]
        name: String,
        /// Publisher domains, comma-separated (e.g. "theguardian.com,guardian.co.uk")
        #[arg(long, value_delimiter = ',', required = true)]
        domains: Vec<String>,
    },
}

// ---------------------------------------------------------------------------
// Entrypoint
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    match cli.command {
        None | Some(Command::Serve) => serve().await,
        Some(Command::Keygen { target }) => keygen(target).await,
    }
}

// ---------------------------------------------------------------------------
// Serve
// ---------------------------------------------------------------------------

async fn serve() {
    let config = ServerConfig::from_env().expect("Failed to load config");

    // Initialise tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level)),
        )
        .json()
        .init();

    tracing::info!(port = config.port, "Starting OA Telemetry Server");

    // Database pool
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations
    oa_telemetry_server::MIGRATOR
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    tracing::info!("Migrations applied");

    // Build state
    let state = OaState::new(pool.clone(), config.clone());

    // Preload domain index
    if let Err(e) = domain_index::refresh_domain_index(&pool, &state.domain_index).await {
        tracing::warn!(error = %e, "Failed to preload domain index (publishers table may not have data yet)");
    }

    // Spawn background domain index refresh
    domain_index::spawn_refresh_task(
        pool.clone(),
        state.domain_index.clone(),
        config.domain_refresh_secs,
    );

    // Build router
    let app = oa_telemetry_server::router()
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!(%addr, "Listening");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app).await.expect("Server error");
}

// ---------------------------------------------------------------------------
// Keygen
// ---------------------------------------------------------------------------

async fn keygen(target: KeygenTarget) {
    let config = ServerConfig::from_env().expect("Failed to load config (need DATABASE_URL)");

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations to ensure tables exist
    oa_telemetry_server::MIGRATOR
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    match target {
        KeygenTarget::Platform { name, platform_id } => {
            let raw_key = generate_raw_key("oat_pk");
            let hash = hash_api_key(&raw_key);

            sqlx::query(
                "INSERT INTO platform_keys (name, platform_id, api_key_hash) VALUES ($1, $2, $3)",
            )
            .bind(&name)
            .bind(&platform_id)
            .bind(&hash)
            .execute(&pool)
            .await
            .expect("Failed to insert platform key (is platform_id unique?)");

            eprintln!("Created platform key:");
            eprintln!("  Name:        {name}");
            eprintln!("  Platform ID: {platform_id}");
            eprintln!();
            eprintln!("  API key (save now — will not be shown again):");
            println!("{raw_key}");
        }
        KeygenTarget::Publisher { name, domains } => {
            if domains.is_empty() {
                eprintln!("Error: --domains required (comma-separated, e.g. \"theguardian.com,guardian.co.uk\")");
                std::process::exit(1);
            }

            let raw_key = generate_raw_key("oat_pub");
            let hash = hash_api_key(&raw_key);

            sqlx::query(
                "INSERT INTO publishers (name, domains, api_key_hash) VALUES ($1, $2, $3)",
            )
            .bind(&name)
            .bind(&domains)
            .bind(&hash)
            .execute(&pool)
            .await
            .expect("Failed to insert publisher");

            eprintln!("Created publisher key:");
            eprintln!("  Name:    {name}");
            eprintln!("  Domains: {}", domains.join(", "));
            eprintln!();
            eprintln!("  API key (save now — will not be shown again):");
            println!("{raw_key}");
        }
    }
}
