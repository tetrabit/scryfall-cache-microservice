pub mod connection;
pub mod queries;

use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::backend::DatabaseBackend;
use crate::db::sqlite::connection::SqlitePool;
use crate::models::card::Card;

pub struct SqliteBackend {
    pool: SqlitePool,
}

impl SqliteBackend {
    pub fn new(pool: SqlitePool) -> Result<Self> {
        // Initialize schema on creation
        connection::init_schema(&pool)?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl DatabaseBackend for SqliteBackend {
    async fn insert_cards_batch(&self, cards: &[Card]) -> Result<()> {
        let pool = self.pool.clone();
        let cards = cards.to_vec(); // Clone to move into spawn_blocking
        tokio::task::spawn_blocking(move || queries::insert_cards_batch(&pool, &cards)).await?
    }

    async fn get_card_by_id(&self, id: Uuid) -> Result<Option<Card>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || queries::get_card_by_id(&pool, id)).await?
    }

    async fn get_cards_by_ids(&self, ids: &[Uuid]) -> Result<Vec<Card>> {
        let pool = self.pool.clone();
        let ids = ids.to_vec();
        tokio::task::spawn_blocking(move || queries::get_cards_by_ids(&pool, &ids)).await?
    }

    async fn search_cards_by_name(&self, name: &str, limit: i64) -> Result<Vec<Card>> {
        let pool = self.pool.clone();
        let name = name.to_string();
        tokio::task::spawn_blocking(move || queries::search_cards_by_name(&pool, &name, limit))
            .await?
    }

    async fn autocomplete_card_names(&self, prefix: &str, limit: i64) -> Result<Vec<String>> {
        let pool = self.pool.clone();
        let prefix = prefix.to_string();
        tokio::task::spawn_blocking(move || queries::autocomplete_card_names(&pool, &prefix, limit))
            .await?
    }

    async fn store_query_cache(
        &self,
        query_hash: &str,
        card_ids: &[Uuid],
        ttl_hours: i32,
    ) -> Result<()> {
        let pool = self.pool.clone();
        let query_hash = query_hash.to_string();
        let card_ids = card_ids.to_vec(); // Clone to move
        tokio::task::spawn_blocking(move || {
            queries::store_query_cache(&pool, &query_hash, &card_ids, ttl_hours)
        })
        .await?
    }

    async fn get_query_cache(&self, query_hash: &str) -> Result<Option<(Vec<Uuid>, i32)>> {
        let pool = self.pool.clone();
        let query_hash = query_hash.to_string();
        tokio::task::spawn_blocking(move || queries::get_query_cache(&pool, &query_hash)).await?
    }

    async fn record_bulk_import(&self, total_cards: i32, source: &str) -> Result<()> {
        let pool = self.pool.clone();
        let source = source.to_string();
        tokio::task::spawn_blocking(move || {
            queries::record_bulk_import(&pool, total_cards, &source)
        })
        .await?
    }

    async fn clean_old_cache_entries(&self, hours: i32) -> Result<u64> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || queries::clean_old_cache_entries(&pool, hours)).await?
    }

    async fn test_connection(&self) -> Result<()> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || connection::test_connection(&pool)).await?
    }

    async fn execute_raw_query(&self, sql: &str, params: &[String]) -> Result<Vec<Card>> {
        let pool = self.pool.clone();
        let sql = sql.to_string();
        let params = params.to_vec();
        tokio::task::spawn_blocking(move || queries::execute_raw_query(&pool, &sql, &params))
            .await?
    }

    async fn count_query(&self, sql: &str, params: &[String]) -> Result<usize> {
        let pool = self.pool.clone();
        let sql = sql.to_string();
        let params = params.to_vec();
        tokio::task::spawn_blocking(move || queries::count_query(&pool, &sql, &params)).await?
    }

    async fn check_bulk_data_loaded(&self) -> Result<bool> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || queries::check_bulk_data_loaded(&pool)).await?
    }

    async fn get_last_bulk_import(&self) -> Result<Option<chrono::NaiveDateTime>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || queries::get_last_bulk_import(&pool)).await?
    }

    async fn get_card_count(&self) -> Result<i64> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || queries::get_card_count(&pool)).await?
    }

    async fn get_cache_entry_count(&self) -> Result<i64> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || queries::get_cache_entry_count(&pool)).await?
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
