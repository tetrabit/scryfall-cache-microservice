use anyhow::Result;
use tracing::{debug, info};
use uuid::Uuid;

use crate::db::Database;
use crate::models::card::Card;
use crate::query::executor::QueryExecutor;
use crate::scryfall::client::ScryfallClient;
use crate::utils::hash::hash_query;

pub struct CacheManager {
    db: Database,
    query_executor: QueryExecutor,
    scryfall_client: ScryfallClient,
}

impl CacheManager {
    pub fn new(db: Database, scryfall_client: ScryfallClient) -> Self {
        let query_executor = QueryExecutor::new(db.clone());

        Self {
            db,
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
        if let Some((card_ids, _total)) = self.db.get_query_cache(&query_hash).await? {
            debug!("Cache hit for query: {} ({} IDs)", query, card_ids.len());

            // Try to fetch cards from cache, but fall back to direct query if it fails
            match self.db.get_cards_by_ids(&card_ids).await {
                Ok(cards) if !cards.is_empty() => {
                    info!("Returned {} cards from cache for query: {}", cards.len(), query);
                    return Ok(cards);
                }
                Err(e) => {
                    // Cache fetch failed, fall through to direct database query
                    debug!("Cache fetch failed ({}), falling back to direct query", e);
                }
                _ => {}
            }
        }

        debug!("Cache miss for query: {}", query);

        // Try to execute query locally first
        match self.query_executor.execute(query, limit).await {
            Ok(cards) if !cards.is_empty() => {
                info!("Returned {} cards from local database for query: {}", cards.len(), query);

                // Store in cache
                let card_ids: Vec<Uuid> = cards.iter().map(|c| c.id).collect();
                self.db.store_query_cache(&query_hash, &card_ids, 24)
                    .await
                    .ok();

                Ok(cards)
            }
            Ok(cards) => {
                // Query succeeded but returned no results
                debug!("Query executor returned {} cards for query: {}", cards.len(), query);
                info!("Querying Scryfall API for: {}", query);
                let cards = self.scryfall_client.search_cards(query).await?;

                if !cards.is_empty() {
                    // Store cards in database
                    self.db.insert_cards_batch(&cards).await?;

                    // Store in cache
                    let card_ids: Vec<Uuid> = cards.iter().map(|c| c.id).collect();
                    self.db.store_query_cache(&query_hash, &card_ids, 24)
                        .await
                        .ok();

                    info!("Returned {} cards from Scryfall API for query: {}", cards.len(), query);
                }

                Ok(cards)
            }
            Err(e) => {
                // Query executor failed with an error
                debug!("Query executor error for query '{}': {}", query, e);
                info!("Querying Scryfall API for: {}", query);
                let cards = self.scryfall_client.search_cards(query).await?;

                if !cards.is_empty() {
                    // Store cards in database
                    self.db.insert_cards_batch(&cards).await?;

                    // Store in cache
                    let card_ids: Vec<Uuid> = cards.iter().map(|c| c.id).collect();
                    self.db.store_query_cache(&query_hash, &card_ids, 24)
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
        if let Ok(Some(card)) = self.db.get_card_by_id(id).await {
            debug!("Found card in local database: {}", card.name);
            return Ok(Some(card));
        }

        // Fall back to Scryfall API
        debug!("Card not in database, querying Scryfall API");
        if let Some(card) = self.scryfall_client.get_card_by_id(id).await? {
            // Store in database
            self.db.insert_cards_batch(&[card.clone()]).await?;
            info!("Fetched and cached card from Scryfall: {}", card.name);
            return Ok(Some(card));
        }

        Ok(None)
    }

    /// Search by card name with caching
    pub async fn search_by_name(&self, name: &str, fuzzy: bool) -> Result<Option<Card>> {
        debug!("Cache search by name: {} (fuzzy={})", name, fuzzy);

        // Try local database first
        let cards = self.db.search_cards_by_name(name, 1).await?;
        if let Some(card) = cards.first() {
            debug!("Found card in local database: {}", card.name);
            return Ok(Some(card.clone()));
        }

        // Fall back to Scryfall API
        debug!("Card not in database, querying Scryfall API");
        if let Some(card) = self.scryfall_client.get_card_by_name(name, fuzzy).await? {
            // Store in database
            self.db.insert_cards_batch(&[card.clone()]).await?;
            info!("Fetched and cached card from Scryfall: {}", card.name);
            return Ok(Some(card));
        }

        Ok(None)
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> Result<CacheStats> {
        let total_cards = self.db.get_card_count().await?;
        let total_cache_entries = self.db.get_cache_entry_count().await?;

        Ok(CacheStats {
            total_cards,
            total_cache_entries,
        })
    }
}

/// Cache statistics
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct CacheStats {
    /// Total number of cards in the database
    pub total_cards: i64,
    /// Total number of cached query results
    pub total_cache_entries: i64,
}
