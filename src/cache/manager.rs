use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::{debug, info};
use uuid::Uuid;

use crate::db::queries::{get_cards_by_ids, get_query_cache, store_query_cache, insert_cards_batch};
use crate::models::card::Card;
use crate::query::executor::QueryExecutor;
use crate::scryfall::client::ScryfallClient;
use crate::utils::hash::hash_query;

pub struct CacheManager {
    pool: PgPool,
    query_executor: QueryExecutor,
    scryfall_client: ScryfallClient,
}

impl CacheManager {
    pub fn new(pool: PgPool, scryfall_client: ScryfallClient) -> Self {
        let query_executor = QueryExecutor::new(pool.clone());

        Self {
            pool,
            query_executor,
            scryfall_client,
        }
    }

    /// Search for cards with caching
    pub async fn search(&self, query: &str, limit: Option<i64>) -> Result<Vec<Card>> {
        debug!("Cache search for query: {}", query);

        // Generate query hash
        let query_hash = hash_query(query);

        // Check cache first
        if let Some((card_ids, _total)) = get_query_cache(&self.pool, &query_hash).await? {
            debug!("Cache hit for query: {}", query);
            let cards = get_cards_by_ids(&self.pool, &card_ids).await?;

            if !cards.is_empty() {
                info!("Returned {} cards from cache for query: {}", cards.len(), query);
                return Ok(cards);
            }
        }

        debug!("Cache miss for query: {}", query);

        // Try to execute query locally first
        match self.query_executor.execute(query, limit).await {
            Ok(cards) if !cards.is_empty() => {
                info!("Returned {} cards from local database for query: {}", cards.len(), query);

                // Store in cache
                let card_ids: Vec<Uuid> = cards.iter().map(|c| c.id).collect();
                store_query_cache(&self.pool, &query_hash, query, &card_ids, cards.len() as i32)
                    .await
                    .ok();

                Ok(cards)
            }
            _ => {
                // Fall back to Scryfall API
                info!("Querying Scryfall API for: {}", query);
                let cards = self.scryfall_client.search_cards(query).await?;

                if !cards.is_empty() {
                    // Store cards in database
                    insert_cards_batch(&self.pool, &cards).await?;

                    // Store in cache
                    let card_ids: Vec<Uuid> = cards.iter().map(|c| c.id).collect();
                    store_query_cache(&self.pool, &query_hash, query, &card_ids, cards.len() as i32)
                        .await
                        .ok();

                    info!("Returned {} cards from Scryfall API for query: {}", cards.len(), query);
                }

                Ok(cards)
            }
        }
    }

    /// Get a card by ID with caching
    pub async fn get_card(&self, id: Uuid) -> Result<Option<Card>> {
        debug!("Cache get card by ID: {}", id);

        // Check local database first
        if let Ok(Some(card)) = crate::db::queries::get_card_by_id(&self.pool, id).await {
            debug!("Found card in local database: {}", card.name);
            return Ok(Some(card));
        }

        // Fall back to Scryfall API
        debug!("Card not in database, querying Scryfall API");
        if let Some(card) = self.scryfall_client.get_card_by_id(id).await? {
            // Store in database
            insert_cards_batch(&self.pool, &[card.clone()]).await?;
            info!("Fetched and cached card from Scryfall: {}", card.name);
            return Ok(Some(card));
        }

        Ok(None)
    }

    /// Search by card name with caching
    pub async fn search_by_name(&self, name: &str, fuzzy: bool) -> Result<Option<Card>> {
        debug!("Cache search by name: {} (fuzzy={})", name, fuzzy);

        // Try local database first
        let cards = crate::db::queries::search_cards_by_name(&self.pool, name, 1).await?;
        if let Some(card) = cards.first() {
            debug!("Found card in local database: {}", card.name);
            return Ok(Some(card.clone()));
        }

        // Fall back to Scryfall API
        debug!("Card not in database, querying Scryfall API");
        if let Some(card) = self.scryfall_client.get_card_by_name(name, fuzzy).await? {
            // Store in database
            insert_cards_batch(&self.pool, &[card.clone()]).await?;
            info!("Fetched and cached card from Scryfall: {}", card.name);
            return Ok(Some(card));
        }

        Ok(None)
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> Result<CacheStats> {
        let card_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cards")
            .fetch_one(&self.pool)
            .await
            .context("Failed to get card count")?;

        let cache_entry_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM query_cache")
            .fetch_one(&self.pool)
            .await
            .context("Failed to get cache entry count")?;

        Ok(CacheStats {
            total_cards: card_count.0,
            total_cache_entries: cache_entry_count.0,
        })
    }
}

#[derive(Debug, serde::Serialize)]
pub struct CacheStats {
    pub total_cards: i64,
    pub total_cache_entries: i64,
}
