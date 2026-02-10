use anyhow::Result;
use async_trait::async_trait;
use std::any::Any;
use std::time::Instant;
use uuid::Uuid;

use crate::db::{Database, DatabaseBackend};
use crate::metrics::registry::{DATABASE_QUERIES_TOTAL, DATABASE_QUERY_DURATION_SECONDS};
use crate::models::card::Card;

/// A thin wrapper around a DatabaseBackend that records basic Prometheus metrics
/// for query counts and durations.
///
/// This keeps performance instrumentation centralized and avoids sprinkling
/// timing code across all backend implementations.
pub struct InstrumentedDatabase {
    inner: Database,
}

impl InstrumentedDatabase {
    pub fn new(inner: Database) -> Self {
        Self { inner }
    }

    fn observe(&self, query_type: &'static str, start: Instant) {
        let seconds = start.elapsed().as_secs_f64();
        DATABASE_QUERIES_TOTAL
            .with_label_values(&[query_type])
            .inc();
        DATABASE_QUERY_DURATION_SECONDS
            .with_label_values(&[query_type])
            .observe(seconds);
    }
}

#[async_trait]
impl DatabaseBackend for InstrumentedDatabase {
    async fn insert_cards_batch(&self, cards: &[Card]) -> Result<()> {
        let start = Instant::now();
        let res = self.inner.insert_cards_batch(cards).await;
        self.observe("insert", start);
        res
    }

    async fn get_card_by_id(&self, id: Uuid) -> Result<Option<Card>> {
        let start = Instant::now();
        let res = self.inner.get_card_by_id(id).await;
        self.observe("select", start);
        res
    }

    async fn get_cards_by_ids(&self, ids: &[Uuid]) -> Result<Vec<Card>> {
        let start = Instant::now();
        let res = self.inner.get_cards_by_ids(ids).await;
        self.observe("select", start);
        res
    }

    async fn search_cards_by_name(&self, name: &str, limit: i64) -> Result<Vec<Card>> {
        let start = Instant::now();
        let res = self.inner.search_cards_by_name(name, limit).await;
        self.observe("select", start);
        res
    }

    async fn autocomplete_card_names(&self, prefix: &str, limit: i64) -> Result<Vec<String>> {
        let start = Instant::now();
        let res = self.inner.autocomplete_card_names(prefix, limit).await;
        self.observe("select", start);
        res
    }

    async fn store_query_cache(&self, query_hash: &str, card_ids: &[Uuid], ttl_hours: i32) -> Result<()> {
        let start = Instant::now();
        let res = self
            .inner
            .store_query_cache(query_hash, card_ids, ttl_hours)
            .await;
        // Upsert/write
        self.observe("insert", start);
        res
    }

    async fn get_query_cache(&self, query_hash: &str) -> Result<Option<(Vec<Uuid>, i32)>> {
        let start = Instant::now();
        let res = self.inner.get_query_cache(query_hash).await;
        self.observe("select", start);
        res
    }

    async fn record_bulk_import(&self, total_cards: i32, source: &str) -> Result<()> {
        let start = Instant::now();
        let res = self.inner.record_bulk_import(total_cards, source).await;
        self.observe("insert", start);
        res
    }

    async fn clean_old_cache_entries(&self, hours: i32) -> Result<u64> {
        let start = Instant::now();
        let res = self.inner.clean_old_cache_entries(hours).await;
        self.observe("delete", start);
        res
    }

    async fn test_connection(&self) -> Result<()> {
        let start = Instant::now();
        let res = self.inner.test_connection().await;
        self.observe("select", start);
        res
    }

    async fn execute_raw_query(&self, sql: &str, params: &[String]) -> Result<Vec<Card>> {
        let start = Instant::now();
        let res = self.inner.execute_raw_query(sql, params).await;
        self.observe("select", start);
        res
    }

    async fn count_query(&self, sql: &str, params: &[String]) -> Result<usize> {
        let start = Instant::now();
        let res = self.inner.count_query(sql, params).await;
        self.observe("select", start);
        res
    }

    async fn check_bulk_data_loaded(&self) -> Result<bool> {
        let start = Instant::now();
        let res = self.inner.check_bulk_data_loaded().await;
        self.observe("select", start);
        res
    }

    async fn get_last_bulk_import(&self) -> Result<Option<chrono::NaiveDateTime>> {
        let start = Instant::now();
        let res = self.inner.get_last_bulk_import().await;
        self.observe("select", start);
        res
    }

    async fn get_card_count(&self) -> Result<i64> {
        let start = Instant::now();
        let res = self.inner.get_card_count().await;
        self.observe("select", start);
        res
    }

    async fn get_cache_entry_count(&self) -> Result<i64> {
        let start = Instant::now();
        let res = self.inner.get_cache_entry_count().await;
        self.observe("select", start);
        res
    }

    fn as_any(&self) -> &dyn Any {
        self.inner.as_any()
    }
}

