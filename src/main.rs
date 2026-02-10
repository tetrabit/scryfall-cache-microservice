mod api;
mod background;
mod cache;
mod circuit_breaker;
mod config;
mod db;
mod errors;
mod graphql;
mod metrics;
mod models;
mod query;
mod scryfall;
mod utils;

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use api::handlers::AppStateInner;
use api::routes::create_router;
use cache::manager::CacheManager;
use cache::redis::{RedisCache, RedisConfig};
use config::Config;
use graphql::create_schema;
use scryfall::bulk_loader::BulkLoader;
use scryfall::client::ScryfallClient;

/// Wait for shutdown signal (SIGTERM or SIGINT)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received SIGTERM signal");
        },
    }

    info!("Starting graceful shutdown...");
}

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

    info!(
        "Starting Scryfall Cache Microservice v{}",
        env!("CARGO_PKG_VERSION")
    );

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

    // Initialize Redis cache (if enabled)
    let redis_cache = if let Some(redis_config) = &config.cache.redis {
        info!("Redis cache is enabled, connecting to {}...", redis_config.url);

        let redis_config_instance = RedisConfig {
            url: redis_config.url.clone(),
            ttl_seconds: redis_config.ttl_seconds,
            max_value_size_mb: redis_config.max_value_size_mb,
        };

        match RedisCache::new(redis_config_instance).await {
            Ok(redis) => {
                redis
                    .test_connection()
                    .await
                    .context("Failed to test Redis connection")?;
                info!("Redis cache connected successfully");
                Some(redis)
            }
            Err(e) => {
                error!("Failed to connect to Redis: {}", e);
                error!("Continuing without Redis cache");
                None
            }
        }
    } else {
        info!("Redis cache is disabled");
        None
    };

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
    let cache_manager = CacheManager::new(
        redis_cache,
        db.clone(),
        scryfall_client,
        config.cache.query_cache_ttl_hours as i32,
    );

    // Initialize query validator
    let query_validator = query::QueryValidator::new(query::QueryLimits::from_env());

    // Wrap cache_manager in Arc for sharing
    let cache_manager_arc = Arc::new(cache_manager);

    // Clone bulk_loader for background job and wrap in Arc
    let bulk_loader_clone = Arc::new(bulk_loader.clone());

    // Create GraphQL schema
    let graphql_schema = create_schema(cache_manager_arc.clone(), bulk_loader_clone.clone());

    // Create application state
    let state = Arc::new(AppStateInner {
        cache_manager: cache_manager_arc,
        bulk_loader: (*bulk_loader_clone).clone(),
        query_validator,
        graphql_schema,
        instance_id: config.server.instance_id.clone(),
    });

    // Start background bulk data refresh job
    let refresh_config = background::bulk_refresh::BulkRefreshConfig::from_env();
    let _refresh_handle = background::start_bulk_refresh_job(bulk_loader_clone, refresh_config);

    // Create router
    let app = create_router(state);

    // Start server
    let addr = config.server_address();
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context("Failed to bind server")?;

    info!("Server listening on {}", addr);

    // Serve with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Server error")?;

    info!("Server shutdown complete");

    Ok(())
}
