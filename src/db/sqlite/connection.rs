use anyhow::{Context, Result};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::path::Path;
use std::time::Duration;

use crate::config::DatabaseConfig;

pub type SqlitePool = Pool<SqliteConnectionManager>;

pub fn create_pool(database_path: &str) -> Result<SqlitePool> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(database_path).parent() {
        std::fs::create_dir_all(parent).context("Failed to create database directory")?;
    }

    let manager = SqliteConnectionManager::file(database_path);

    Pool::builder()
        .max_size(15) // SQLite doesn't handle as many connections as Postgres
        .connection_timeout(Duration::from_secs(30))
        .build(manager)
        .context("Failed to create SQLite connection pool")
}

pub fn test_connection(pool: &SqlitePool) -> Result<()> {
    let conn = pool.get().context("Failed to get connection from pool")?;
    conn.query_row("SELECT 1", params![], |_| Ok(()))
        .context("Failed to test database connection")?;
    Ok(())
}

pub fn init_schema(pool: &SqlitePool) -> Result<()> {
    let conn = pool.get().context("Failed to get connection from pool")?;

    // Create cards table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS cards (
            id TEXT PRIMARY KEY,
            oracle_id TEXT,
            name TEXT NOT NULL,
            mana_cost TEXT,
            cmc REAL,
            type_line TEXT,
            oracle_text TEXT,
            colors TEXT,
            color_identity TEXT,
            set_code TEXT,
            set_name TEXT,
            collector_number TEXT,
            rarity TEXT,
            power TEXT,
            toughness TEXT,
            loyalty TEXT,
            keywords TEXT,
            prices TEXT,
            image_uris TEXT,
            card_faces TEXT,
            legalities TEXT,
            released_at TEXT,
            raw_json TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        params![],
    )
    .context("Failed to create cards table")?;

    // Create query_cache table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS query_cache (
            query_hash TEXT PRIMARY KEY,
            card_ids TEXT NOT NULL,
            ttl_hours INTEGER NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            expires_at TEXT NOT NULL
        )
        "#,
        params![],
    )
    .context("Failed to create query_cache table")?;

    // Create bulk_imports table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS bulk_imports (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            total_cards INTEGER NOT NULL,
            source TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        params![],
    )
    .context("Failed to create bulk_imports table")?;

    // Create indexes for performance
    // Note: SQLite doesn't support GIN indexes like PostgreSQL, so we use standard B-tree indexes

    // Core indexes for common queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_cards_name ON cards(name)",
        params![],
    )
    .context("Failed to create name index")?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_cards_oracle_id ON cards(oracle_id)",
        params![],
    )
    .context("Failed to create oracle_id index")?;

    // Phase 2 Performance Indexes - Added for 2-3x query speedup
    // These indexes optimize common Scryfall query patterns

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_cards_colors ON cards(colors)",
        params![],
    )
    .context("Failed to create colors index")?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_cards_color_identity ON cards(color_identity)",
        params![],
    )
    .context("Failed to create color_identity index")?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_cards_cmc ON cards(cmc)",
        params![],
    )
    .context("Failed to create cmc index")?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_cards_type_line ON cards(type_line)",
        params![],
    )
    .context("Failed to create type_line index")?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_cards_set_code ON cards(set_code)",
        params![],
    )
    .context("Failed to create set_code index")?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_cards_rarity ON cards(rarity)",
        params![],
    )
    .context("Failed to create rarity index")?;

    // Composite indexes for common query combinations
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_cards_set_rarity ON cards(set_code, rarity)",
        params![],
    )
    .context("Failed to create set_rarity composite index")?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_cards_set_collector ON cards(set_code, collector_number)",
        params![],
    )
    .context("Failed to create set_collector composite index")?;

    // Query cache index
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_query_cache_expires ON query_cache(expires_at)",
        params![],
    )
    .context("Failed to create expires_at index")?;

    Ok(())
}
