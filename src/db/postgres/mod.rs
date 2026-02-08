pub mod connection;
pub mod queries;

use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::backend::DatabaseBackend;
use crate::models::card::Card;

pub struct PostgresBackend {
    pool: PgPool,
}

impl PostgresBackend {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl DatabaseBackend for PostgresBackend {
    async fn insert_cards_batch(&self, cards: &[Card]) -> Result<()> {
        queries::insert_cards_batch(&self.pool, cards).await
    }

    async fn get_card_by_id(&self, id: Uuid) -> Result<Option<Card>> {
        queries::get_card_by_id(&self.pool, id).await
    }

    async fn get_cards_by_ids(&self, ids: &[Uuid]) -> Result<Vec<Card>> {
        queries::get_cards_by_ids(&self.pool, ids).await
    }

    async fn search_cards_by_name(&self, name: &str, limit: i64) -> Result<Vec<Card>> {
        queries::search_cards_by_name(&self.pool, name, limit).await
    }

    async fn store_query_cache(
        &self,
        query_hash: &str,
        card_ids: &[Uuid],
        ttl_hours: i32,
    ) -> Result<()> {
        queries::store_query_cache(&self.pool, query_hash, card_ids, ttl_hours).await
    }

    async fn get_query_cache(&self, query_hash: &str) -> Result<Option<(Vec<Uuid>, i32)>> {
        queries::get_query_cache(&self.pool, query_hash).await
    }

    async fn record_bulk_import(&self, total_cards: i32, source: &str) -> Result<()> {
        queries::record_bulk_import(&self.pool, total_cards, source).await
    }

    async fn clean_old_cache_entries(&self, hours: i32) -> Result<u64> {
        queries::clean_old_cache_entries(&self.pool, hours).await
    }

    async fn test_connection(&self) -> Result<()> {
        connection::test_connection(&self.pool).await
    }

    async fn execute_raw_query(
        &self,
        sql: &str,
        params: &[String],
    ) -> Result<Vec<Card>> {
        queries::execute_raw_query(&self.pool, sql, params).await
    }

    async fn check_bulk_data_loaded(&self) -> Result<bool> {
        queries::check_bulk_data_loaded(&self.pool).await
    }

    async fn get_last_bulk_import(&self) -> Result<Option<chrono::NaiveDateTime>> {
        queries::get_last_bulk_import(&self.pool).await
    }

    async fn get_card_count(&self) -> Result<i64> {
        queries::get_card_count(&self.pool).await
    }

    async fn get_cache_entry_count(&self) -> Result<i64> {
        queries::get_cache_entry_count(&self.pool).await
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
