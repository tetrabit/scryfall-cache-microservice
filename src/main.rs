mod api;
mod cache;
mod config;
mod db;
mod metrics;
mod models;
mod query;
mod scryfall;
mod utils;

use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use api::handlers::AppStateInner;
use api::routes::create_router;
use cache::manager::CacheManager;
use config::Config;
use scryfall::bulk_loader::BulkLoader;
use scryfall::client::ScryfallClient;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,scryfall_cache=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Scryfall Cache Microservice v{}", env!("CARGO_PKG_VERSION"));

    // Initialize metrics
    metrics::registry::init_metrics();
    info!("Metrics registry initialized");

    // Load configuration
    let config = Config::from_env().context("Failed to load configuration")?;
    info!("Configuration loaded successfully");

    // Initialize database backend
    info!("Connecting to database...");
    let db = db::init_database(&config.database)
        .await
        .context("Failed to initialize database")?;
    
    db.test_connection()
        .await
        .context("Failed to test database connection")?;
    info!("Database connection established");

    // Run migrations (PostgreSQL only)
    #[cfg(feature = "postgres")]
    {
        if let Some(pg_backend) = db.as_any().downcast_ref::<db::PostgresBackend>() {
            db::schema::run_migrations(pg_backend.pool())
                .await
                .context("Failed to run database migrations")?;
        }
    }

    // Initialize Scryfall client
    let scryfall_client = ScryfallClient::new(&config.scryfall);

    // Initialize bulk loader
    let bulk_loader = BulkLoader::new(db.clone(), config.scryfall.clone());

    // Load bulk data if needed
    if bulk_loader.should_load().await? {
        info!("Loading bulk data...");
        if let Err(e) = bulk_loader.load().await {
            error!("Failed to load bulk data: {}", e);
            error!("Continuing without bulk data - will rely on API fallback");
        }
    } else {
        info!("Bulk data is up to date, skipping load");
    }

    // Initialize cache manager
    let cache_manager = CacheManager::new(db.clone(), scryfall_client);

    // Create application state
    let state = Arc::new(AppStateInner {
        cache_manager,
        bulk_loader,
    });

    // Create router
    let app = create_router(state);

    // Start server
    let addr = config.server_address();
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context("Failed to bind server")?;

    info!("Server listening on {}", addr);

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}
