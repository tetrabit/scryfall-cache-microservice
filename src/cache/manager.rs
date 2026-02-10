use anyhow::Result;
use std::collections::HashSet;
use tracing::{debug, info};
use uuid::Uuid;

use crate::cache::redis::RedisCache;
use crate::db::Database;
use crate::metrics::registry::{CACHE_HITS_TOTAL, CACHE_MISSES_TOTAL};
use crate::models::card::Card;
use crate::query::executor::QueryExecutor;
use crate::scryfall::client::ScryfallClient;
use crate::utils::hash::hash_query;

pub struct CacheManager {
    redis: Option<RedisCache>,
    db: Database,
    query_executor: QueryExecutor,
    scryfall_client: ScryfallClient,
    query_cache_ttl_hours: i32,
}

impl CacheManager {
    pub fn new(
        redis: Option<RedisCache>,
        db: Database,
        scryfall_client: ScryfallClient,
        query_cache_ttl_hours: i32,
    ) -> Self {
        let query_executor = QueryExecutor::new(db.clone());

        Self {
            redis,
            db,
            query_executor,
            scryfall_client,
            query_cache_ttl_hours,
        }
    }

    pub async fn test_database_connection(&self) -> Result<()> {
        self.db.test_connection().await
    }

    pub async fn test_redis_connection(&self) -> Result<()> {
        if let Some(redis) = &self.redis {
            redis.test_connection().await
        } else {
            Ok(())
        }
    }

    /// Search for cards with caching
    pub async fn search(&self, query: &str, limit: Option<i64>) -> Result<Vec<Card>> {
        debug!("Cache search for query: {}", query);

        // 1. Check Redis cache first (if enabled)
        if let Some(redis) = &self.redis {
            if let Ok(Some(card_ids)) = redis.get_query_results(query).await {
                debug!("Redis cache hit for query: {} ({} IDs)", query, card_ids.len());

                // Try to fetch cards from database
                match self.db.get_cards_by_ids(&card_ids).await {
                    Ok(cards) if !cards.is_empty() => {
                        info!(
                            "Returned {} cards from Redis cache for query: {}",
                            cards.len(),
                            query
                        );
                        return Ok(cards);
                    }
                    _ => {
                        debug!("Redis had IDs but database fetch failed, falling back");
                    }
                }
            }
        }

        // 2. Check database query cache
        let query_hash = hash_query(query);
        if let Some((card_ids, _total)) = self.db.get_query_cache(&query_hash).await? {
            debug!("Database query cache hit for query: {} ({} IDs)", query, card_ids.len());

            // Try to fetch cards from database
            match self.db.get_cards_by_ids(&card_ids).await {
                Ok(cards) if !cards.is_empty() => {
                    CACHE_HITS_TOTAL.with_label_values(&["query_cache"]).inc();
                    info!(
                        "Returned {} cards from database cache for query: {}",
                        cards.len(),
                        query
                    );

                    // Store in Redis for faster access next time
                    if let Some(redis) = &self.redis {
                        redis.set_query_results(query, &card_ids).await.ok();
                    }

                    return Ok(cards);
                }
                Err(e) => {
                    debug!("Cache fetch failed ({}), falling back to direct query", e);
                }
                _ => {}
            }

            CACHE_MISSES_TOTAL.with_label_values(&["query_cache"]).inc();
        } else {
            CACHE_MISSES_TOTAL.with_label_values(&["query_cache"]).inc();
        }

        debug!("Cache miss for query: {}", query);

        // 3. Try to execute query locally against database
        match self.query_executor.execute(query, limit).await {
            Ok(cards) if !cards.is_empty() => {
                CACHE_HITS_TOTAL.with_label_values(&["database"]).inc();
                info!(
                    "Returned {} cards from local database for query: {}",
                    cards.len(),
                    query
                );

                // Store in both caches
                let card_ids: Vec<Uuid> = cards.iter().map(|c| c.id).collect();

                // Store in database query cache
                self.db
                    .store_query_cache(&query_hash, &card_ids, self.query_cache_ttl_hours)
                    .await
                    .ok();

                // Store in Redis cache
                if let Some(redis) = &self.redis {
                    redis.set_query_results(query, &card_ids).await.ok();
                }

                Ok(cards)
            }
            Ok(cards) => {
                CACHE_MISSES_TOTAL.with_label_values(&["database"]).inc();
                // Query succeeded but returned no results
                debug!(
                    "Query executor returned {} cards for query: {}",
                    cards.len(),
                    query
                );
                info!("Querying Scryfall API for: {}", query);
                let cards = self.scryfall_client.search_cards(query).await?;

                if !cards.is_empty() {
                    CACHE_HITS_TOTAL.with_label_values(&["api"]).inc();
                    // Store cards in database
                    self.db.insert_cards_batch(&cards).await?;

                    // Store in both caches
                    let card_ids: Vec<Uuid> = cards.iter().map(|c| c.id).collect();

                    // Store in database query cache
                    self.db
                        .store_query_cache(&query_hash, &card_ids, self.query_cache_ttl_hours)
                        .await
                        .ok();

                    // Store in Redis cache
                    if let Some(redis) = &self.redis {
                        redis.set_query_results(query, &card_ids).await.ok();
                        redis.set_cards(&cards).await.ok();
                    }

                    info!(
                        "Returned {} cards from Scryfall API for query: {}",
                        cards.len(),
                        query
                    );
                } else {
                    CACHE_MISSES_TOTAL.with_label_values(&["api"]).inc();
                }

                Ok(cards)
            }
            Err(e) => {
                CACHE_MISSES_TOTAL.with_label_values(&["database"]).inc();
                // Query executor failed with an error
                debug!("Query executor error for query '{}': {}", query, e);
                info!("Querying Scryfall API for: {}", query);
                let cards = self.scryfall_client.search_cards(query).await?;

                if !cards.is_empty() {
                    CACHE_HITS_TOTAL.with_label_values(&["api"]).inc();
                    // Store cards in database
                    self.db.insert_cards_batch(&cards).await?;

                    // Store in both caches
                    let card_ids: Vec<Uuid> = cards.iter().map(|c| c.id).collect();

                    // Store in database query cache
                    self.db
                        .store_query_cache(&query_hash, &card_ids, self.query_cache_ttl_hours)
                        .await
                        .ok();

                    // Store in Redis cache
                    if let Some(redis) = &self.redis {
                        redis.set_query_results(query, &card_ids).await.ok();
                        redis.set_cards(&cards).await.ok();
                    }

                    info!(
                        "Returned {} cards from Scryfall API for query: {}",
                        cards.len(),
                        query
                    );
                } else {
                    CACHE_MISSES_TOTAL.with_label_values(&["api"]).inc();
                }

                Ok(cards)
            }
        }
    }

    /// Search for cards with pagination (optimized - fetches only requested page)
    pub async fn search_paginated(
        &self,
        query: &str,
        page: usize,
        page_size: usize,
    ) -> Result<(Vec<Card>, usize)> {
        debug!(
            "Cache paginated search for query: {} (page {}, page_size {})",
            query, page, page_size
        );

        // For paginated queries, we can't rely on query_cache as easily
        // since it stores all card IDs but pagination happens at query level
        // Instead, we directly use the paginated query executor

        match self
            .query_executor
            .execute_paginated(query, page, page_size)
            .await
        {
            Ok((cards, total)) => {
                if !cards.is_empty() || total > 0 {
                    CACHE_HITS_TOTAL.with_label_values(&["database"]).inc();
                    info!(
                        "Returned {} cards from local database for query: {} (page {}/{})",
                        cards.len(),
                        query,
                        page,
                        total.div_ceil(page_size)
                    );
                    Ok((cards, total))
                } else {
                    CACHE_MISSES_TOTAL.with_label_values(&["database"]).inc();
                    // Query returned no results - fall back to Scryfall API
                    debug!("Local query returned no results, querying Scryfall API");
                    info!("Querying Scryfall API for: {}", query);
                    let cards = self.scryfall_client.search_cards(query).await?;

                    if !cards.is_empty() {
                        CACHE_HITS_TOTAL.with_label_values(&["api"]).inc();
                        // Store cards in database
                        self.db.insert_cards_batch(&cards).await?;
                        info!(
                            "Stored {} cards from Scryfall API for query: {}",
                            cards.len(),
                            query
                        );
                    } else {
                        CACHE_MISSES_TOTAL.with_label_values(&["api"]).inc();
                    }

                    // For Scryfall API fallback, apply pagination in-memory
                    // since we fetched all results
                    let total = cards.len();
                    let start = (page.saturating_sub(1)) * page_size;
                    let end = (start + page_size).min(total);

                    let paginated_cards = if start < total {
                        cards[start..end].to_vec()
                    } else {
                        Vec::new()
                    };

                    Ok((paginated_cards, total))
                }
            }
            Err(e) => {
                CACHE_MISSES_TOTAL.with_label_values(&["database"]).inc();
                // Query executor failed - fall back to Scryfall API
                debug!("Query executor error: {}", e);
                info!("Querying Scryfall API for: {}", query);
                let cards = self.scryfall_client.search_cards(query).await?;

                if !cards.is_empty() {
                    CACHE_HITS_TOTAL.with_label_values(&["api"]).inc();
                    // Store cards in database
                    self.db.insert_cards_batch(&cards).await?;
                    info!(
                        "Stored {} cards from Scryfall API for query: {}",
                        cards.len(),
                        query
                    );
                } else {
                    CACHE_MISSES_TOTAL.with_label_values(&["api"]).inc();
                }

                // Apply pagination in-memory
                let total = cards.len();
                let start = (page.saturating_sub(1)) * page_size;
                let end = (start + page_size).min(total);

                let paginated_cards = if start < total {
                    cards[start..end].to_vec()
                } else {
                    Vec::new()
                };

                Ok((paginated_cards, total))
            }
        }
    }

    /// Get a card by ID with caching
    pub async fn get_card(&self, id: Uuid) -> Result<Option<Card>> {
        debug!("Cache get card by ID: {}", id);

        // 1. Check Redis cache first (if enabled)
        if let Some(redis) = &self.redis {
            if let Ok(Some(card)) = redis.get_card(id).await {
                debug!("Found card in Redis cache: {}", card.name);
                return Ok(Some(card));
            }
        }

        // 2. Check local database
        if let Ok(Some(card)) = self.db.get_card_by_id(id).await {
            CACHE_HITS_TOTAL.with_label_values(&["database"]).inc();
            debug!("Found card in local database: {}", card.name);

            // Store in Redis for faster access next time
            if let Some(redis) = &self.redis {
                redis.set_card(&card).await.ok();
            }

            return Ok(Some(card));
        }

        CACHE_MISSES_TOTAL.with_label_values(&["database"]).inc();

        // 3. Fall back to Scryfall API
        debug!("Card not in database, querying Scryfall API");
        if let Some(card) = self.scryfall_client.get_card_by_id(id).await? {
            CACHE_HITS_TOTAL.with_label_values(&["api"]).inc();

            // Store in database
            self.db.insert_cards_batch(&[card.clone()]).await?;

            // Store in Redis cache
            if let Some(redis) = &self.redis {
                redis.set_card(&card).await.ok();
            }

            info!("Fetched and cached card from Scryfall: {}", card.name);
            return Ok(Some(card));
        }

        CACHE_MISSES_TOTAL.with_label_values(&["api"]).inc();
        Ok(None)
    }

    /// Search by card name with caching
    pub async fn search_by_name(&self, name: &str, fuzzy: bool) -> Result<Option<Card>> {
        debug!("Cache search by name: {} (fuzzy={})", name, fuzzy);

        // Try local database first
        let cards = self.db.search_cards_by_name(name, 1).await?;
        if let Some(card) = cards.first() {
            CACHE_HITS_TOTAL.with_label_values(&["database"]).inc();
            debug!("Found card in local database: {}", card.name);
            return Ok(Some(card.clone()));
        }

        CACHE_MISSES_TOTAL.with_label_values(&["database"]).inc();

        // Fall back to Scryfall API
        debug!("Card not in database, querying Scryfall API");
        if let Some(card) = self.scryfall_client.get_card_by_name(name, fuzzy).await? {
            CACHE_HITS_TOTAL.with_label_values(&["api"]).inc();
            // Store in database
            self.db.insert_cards_batch(&[card.clone()]).await?;
            info!("Fetched and cached card from Scryfall: {}", card.name);
            return Ok(Some(card));
        }

        CACHE_MISSES_TOTAL.with_label_values(&["api"]).inc();
        Ok(None)
    }

    /// Autocomplete card names by prefix (case-insensitive)
    /// Returns up to 20 card names that start with the given prefix
    pub async fn autocomplete(&self, prefix: &str) -> Result<Vec<String>> {
        debug!("Autocomplete request: prefix='{}'", prefix);

        if prefix.len() < 2 {
            // Don't autocomplete for very short queries to avoid returning too many results
            return Ok(Vec::new());
        }

        // 1. Check Redis cache first (if enabled)
        if let Some(redis) = &self.redis {
            if let Ok(Some(names)) = redis.get_autocomplete(prefix).await {
                debug!("Autocomplete Redis cache hit for prefix '{}'", prefix);
                return Ok(names);
            }
        }

        // 2. Query the database for matching card names
        let names = self.db.autocomplete_card_names(prefix, 20).await?;

        // Store in Redis for faster access next time
        if let Some(redis) = &self.redis {
            redis.set_autocomplete(prefix, &names).await.ok();
        }

        info!(
            "Autocomplete returned {} names for prefix '{}'",
            names.len(),
            prefix
        );
        Ok(names)
    }

    /// Fetch multiple cards by IDs in one call.
    /// - Reads from the local DB first.
    /// - Optionally fetches missing cards from Scryfall using /cards/collection (chunked) and stores them.
    /// Returns (cards_in_request_order, missing_ids_unique).
    pub async fn get_cards_batch(
        &self,
        ids: &[Uuid],
        fetch_missing: bool,
    ) -> Result<(Vec<Card>, Vec<Uuid>)> {
        if ids.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let found_cards = self.db.get_cards_by_ids(ids).await?;
        let mut by_id = std::collections::HashMap::with_capacity(found_cards.len());
        for card in found_cards {
            by_id.insert(card.id, card);
        }

        let mut missing_ids = Vec::new();
        let mut seen_missing = HashSet::new();
        for id in ids {
            if !by_id.contains_key(id) && seen_missing.insert(*id) {
                missing_ids.push(*id);
            }
        }

        if missing_ids.is_empty() {
            CACHE_HITS_TOTAL.with_label_values(&["database"]).inc();
        } else {
            CACHE_MISSES_TOTAL.with_label_values(&["database"]).inc();
        }

        if fetch_missing && !missing_ids.is_empty() {
            let fetched = self
                .scryfall_client
                .get_cards_by_ids_collection(&missing_ids)
                .await?;

            if !fetched.is_empty() {
                CACHE_HITS_TOTAL.with_label_values(&["api"]).inc();
                self.db.insert_cards_batch(&fetched).await?;
                for card in fetched {
                    by_id.insert(card.id, card);
                }
            } else {
                CACHE_MISSES_TOTAL.with_label_values(&["api"]).inc();
            }

            // Recompute missing IDs after attempted fetch.
            let mut still_missing = Vec::new();
            let mut seen_still = HashSet::new();
            for id in ids {
                if !by_id.contains_key(id) && seen_still.insert(*id) {
                    still_missing.push(*id);
                }
            }
            missing_ids = still_missing;
        }

        let cards_in_order: Vec<Card> = ids.iter().filter_map(|id| by_id.get(id).cloned()).collect();
        Ok((cards_in_order, missing_ids))
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
