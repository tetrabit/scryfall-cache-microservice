use anyhow::Result;
use async_trait::async_trait;
use std::any::Any;
use uuid::Uuid;

use crate::models::card::Card;

/// Database backend trait for abstracting PostgreSQL and SQLite
#[async_trait]
pub trait DatabaseBackend: Send + Sync {
    /// Insert a batch of cards into the database
    async fn insert_cards_batch(&self, cards: &[Card]) -> Result<()>;

    /// Get a card by ID
    async fn get_card_by_id(&self, id: Uuid) -> Result<Option<Card>>;

    /// Get multiple cards by IDs
    async fn get_cards_by_ids(&self, ids: &[Uuid]) -> Result<Vec<Card>>;

    /// Search cards by name (fuzzy search)
    async fn search_cards_by_name(&self, name: &str, limit: i64) -> Result<Vec<Card>>;

    /// Autocomplete card names by prefix (case-insensitive)
    /// Returns up to `limit` card names that start with the given prefix, sorted alphabetically
    async fn autocomplete_card_names(&self, prefix: &str, limit: i64) -> Result<Vec<String>>;

    /// Store a query result in the cache
    async fn store_query_cache(
        &self,
        query_hash: &str,
        card_ids: &[Uuid],
        ttl_hours: i32,
    ) -> Result<()>;

    /// Get cached query results
    async fn get_query_cache(&self, query_hash: &str) -> Result<Option<(Vec<Uuid>, i32)>>;

    /// Record a bulk import operation
    async fn record_bulk_import(
        &self,
        total_cards: i32,
        source: &str,
    ) -> Result<()>;

    /// Clean old cache entries
    async fn clean_old_cache_entries(&self, hours: i32) -> Result<u64>;

    /// Test database connection
    async fn test_connection(&self) -> Result<()>;

    /// Execute a raw SQL query and return cards
    /// This is primarily for PostgreSQL; SQLite support may be limited
    async fn execute_raw_query(
        &self,
        sql: &str,
        params: &[String],
    ) -> Result<Vec<Card>>;

    /// Execute a COUNT query and return the result
    async fn count_query(
        &self,
        sql: &str,
        params: &[String],
    ) -> Result<usize>;

    /// Check if bulk data is loaded (count of cards > 0)
    async fn check_bulk_data_loaded(&self) -> Result<bool>;

    /// Get the timestamp of the last bulk import
    async fn get_last_bulk_import(&self) -> Result<Option<chrono::NaiveDateTime>>;

    /// Get the total count of cards in the database
    async fn get_card_count(&self) -> Result<i64>;

    /// Get the total count of query cache entries
    async fn get_cache_entry_count(&self) -> Result<i64>;

    /// Return self as Any for downcasting
    fn as_any(&self) -> &dyn Any;
}
