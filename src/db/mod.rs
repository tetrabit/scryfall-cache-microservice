pub mod backend;
pub mod schema;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "sqlite")]
pub mod sqlite;

use anyhow::Result;
use std::sync::Arc;

#[cfg(feature = "postgres")]
pub use postgres::PostgresBackend;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteBackend;

pub use backend::DatabaseBackend;

/// Database connection type - polymorphic over backends
pub type Database = Arc<dyn DatabaseBackend>;

/// Initialize database backend based on configuration
#[cfg(feature = "postgres")]
pub async fn init_database(config: &crate::config::DatabaseConfig) -> Result<Database> {
    tracing::info!("Initializing PostgreSQL backend");
    let pool = postgres::connection::create_pool(config).await?;
    postgres::connection::test_connection(&pool).await?;
    let backend = PostgresBackend::new(pool);
    Ok(Arc::new(backend) as Database)
}

#[cfg(feature = "sqlite")]
pub async fn init_database(config: &crate::config::DatabaseConfig) -> Result<Database> {
    tracing::info!("Initializing SQLite backend");
    let database_path = std::env::var("SQLITE_PATH")
        .unwrap_or_else(|_| "./data/scryfall-cache.db".to_string());
    
    let pool = sqlite::connection::create_pool(&database_path)?;
    let backend = SqliteBackend::new(pool)?;
    Ok(Arc::new(backend) as Database)
}
