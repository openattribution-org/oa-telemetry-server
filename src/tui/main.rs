mod app;
mod backend;
mod event;
mod tabs;
mod ui;

use std::io;
use std::time::Duration;

use clap::Parser;
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use sqlx::postgres::PgPoolOptions;

use backend::{Backend, DbBackend, HttpBackend};

#[derive(Parser)]
#[command(name = "oa-tui", version, about = "OpenAttribution TUI Admin Panel")]
struct Cli {
    /// PostgreSQL connection URL for direct database access
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Server base URL for HTTP API access (e.g. https://oa-telemetry-server.fly.dev)
    #[arg(long, env = "OA_SERVER_URL")]
    server_url: Option<String>,

    /// API key for HTTP mode authentication
    #[arg(long, env = "OA_API_KEY")]
    api_key: Option<String>,

    /// Tick rate in milliseconds for live updates
    #[arg(long, default_value = "1000")]
    tick_rate_ms: u64,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    if cli.database_url.is_none() && cli.server_url.is_none() {
        eprintln!("Error: provide --database-url and/or --server-url");
        eprintln!("  --database-url / DATABASE_URL   Direct PostgreSQL access");
        eprintln!("  --server-url / OA_SERVER_URL    HTTP API access");
        std::process::exit(1);
    }

    // Set up backends
    let db = if let Some(url) = &cli.database_url {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await?;
        // Run migrations
        oa_telemetry_server::MIGRATOR.run(&pool).await?;
        Some(DbBackend { pool })
    } else {
        None
    };

    let http = cli.server_url.map(|url| HttpBackend {
        client: reqwest::Client::new(),
        base_url: url,
        api_key: cli.api_key,
    });

    let backend_ref = Backend { db, http };

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    // Run app
    let app_state = app::App::new(backend_ref, Duration::from_millis(cli.tick_rate_ms));
    let result = event::run_event_loop(app_state, terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    result
}
