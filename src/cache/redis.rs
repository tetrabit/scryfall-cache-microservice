use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::metrics::registry::{CACHE_HITS_TOTAL, CACHE_MISSES_TOTAL};
use crate::models::card::Card;
use crate::utils::hash::hash_query;

#[cfg(feature = "redis_cache")]
use redis::{aio::ConnectionManager, AsyncCommands, Client};

/// Redis cache configuration
#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub ttl_seconds: u64,
    pub max_value_size_mb: usize,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            ttl_seconds: 3600, // 1 hour
            max_value_size_mb: 10,
        }
    }
}

/// Cache entry wrapper with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry<T> {
    data: T,
    cached_at: i64,
}

impl<T> CacheEntry<T> {
    fn new(data: T) -> Self {
        Self {
            data,
            cached_at: chrono::Utc::now().timestamp(),
        }
    }
}

/// Redis cache client wrapper
#[cfg(feature = "redis_cache")]
pub struct RedisCache {
    client: ConnectionManager,
    config: RedisConfig,
}

#[cfg(feature = "redis_cache")]
impl RedisCache {
    /// Create a new Redis cache client
    pub async fn new(config: RedisConfig) -> Result<Self> {
        let client = Client::open(config.url.as_str())
            .context("Failed to create Redis client")?;

        let connection_manager = ConnectionManager::new(client)
            .await
            .context("Failed to connect to Redis")?;

        debug!("Redis cache connected successfully");

        Ok(Self {
            client: connection_manager,
            config,
        })
    }

    /// Test the Redis connection
    pub async fn test_connection(&self) -> Result<()> {
        let mut conn = self.client.clone();
        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
            .context("Redis PING failed")?;
        Ok(())
    }

    /// Get query results from cache
    pub async fn get_query_results(&self, query: &str) -> Result<Option<Vec<Uuid>>> {
        let key = format!("query:{}", hash_query(query));

        match self.get_value::<Vec<Uuid>>(&key).await {
            Ok(Some(card_ids)) => {
                CACHE_HITS_TOTAL.with_label_values(&["redis"]).inc();
                debug!("Redis cache hit for query: {} ({} IDs)", query, card_ids.len());
                Ok(Some(card_ids))
            }
            Ok(None) => {
                CACHE_MISSES_TOTAL.with_label_values(&["redis"]).inc();
                debug!("Redis cache miss for query: {}", query);
                Ok(None)
            }
            Err(e) => {
                error!("Redis query cache get error: {}", e);
                CACHE_MISSES_TOTAL.with_label_values(&["redis"]).inc();
                Ok(None) // Fail gracefully
            }
        }
    }

    /// Store query results in cache
    pub async fn set_query_results(&self, query: &str, card_ids: &[Uuid]) -> Result<()> {
        let key = format!("query:{}", hash_query(query));
        let card_ids_vec: Vec<Uuid> = card_ids.to_vec();
        self.set_value(&key, &card_ids_vec, Some(self.config.ttl_seconds)).await
    }

    /// Get a card by ID
    pub async fn get_card(&self, id: Uuid) -> Result<Option<Card>> {
        let key = format!("card:{}", id);

        match self.get_value::<Card>(&key).await {
            Ok(Some(card)) => {
                CACHE_HITS_TOTAL.with_label_values(&["redis"]).inc();
                debug!("Redis cache hit for card: {}", id);
                Ok(Some(card))
            }
            Ok(None) => {
                CACHE_MISSES_TOTAL.with_label_values(&["redis"]).inc();
                Ok(None)
            }
            Err(e) => {
                error!("Redis card cache get error: {}", e);
                CACHE_MISSES_TOTAL.with_label_values(&["redis"]).inc();
                Ok(None) // Fail gracefully
            }
        }
    }

    /// Store a card in cache
    pub async fn set_card(&self, card: &Card) -> Result<()> {
        let key = format!("card:{}", card.id);
        self.set_value(&key, card, Some(self.config.ttl_seconds)).await
    }

    /// Get multiple cards by IDs
    pub async fn get_cards(&self, ids: &[Uuid]) -> Result<Vec<Card>> {
        let mut cards = Vec::new();

        for id in ids {
            if let Some(card) = self.get_card(*id).await? {
                cards.push(card);
            }
        }

        Ok(cards)
    }

    /// Store multiple cards in cache
    pub async fn set_cards(&self, cards: &[Card]) -> Result<()> {
        for card in cards {
            // Best effort - don't fail if one card fails to cache
            if let Err(e) = self.set_card(card).await {
                warn!("Failed to cache card {}: {}", card.id, e);
            }
        }
        Ok(())
    }

    /// Get autocomplete results
    pub async fn get_autocomplete(&self, prefix: &str) -> Result<Option<Vec<String>>> {
        let key = format!("autocomplete:{}", prefix.to_lowercase());
        self.get_value::<Vec<String>>(&key).await
    }

    /// Store autocomplete results
    pub async fn set_autocomplete(&self, prefix: &str, names: &[String]) -> Result<()> {
        let key = format!("autocomplete:{}", prefix.to_lowercase());
        let names_vec: Vec<String> = names.to_vec();
        // Autocomplete results expire faster (10 minutes)
        self.set_value(&key, &names_vec, Some(600)).await
    }

    /// Invalidate all caches (e.g., after bulk data reload)
    pub async fn invalidate_all(&self) -> Result<()> {
        let mut conn = self.client.clone();
        redis::cmd("FLUSHDB")
            .query_async::<_, ()>(&mut conn)
            .await
            .context("Failed to flush Redis database")?;

        debug!("Redis cache invalidated");
        Ok(())
    }

    /// Get cache stats
    pub async fn get_stats(&self) -> Result<RedisStats> {
        let mut conn = self.client.clone();

        let info: String = redis::cmd("INFO")
            .arg("stats")
            .query_async(&mut conn)
            .await
            .context("Failed to get Redis stats")?;

        // Parse keyspace_hits and keyspace_misses from INFO output
        let mut hits = 0u64;
        let mut misses = 0u64;

        for line in info.lines() {
            if line.starts_with("keyspace_hits:") {
                hits = line.split(':').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            } else if line.starts_with("keyspace_misses:") {
                misses = line.split(':').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            }
        }

        let total = hits + misses;
        let hit_rate = if total > 0 {
            (hits as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Ok(RedisStats {
            hits,
            misses,
            hit_rate,
        })
    }

    /// Generic get value from cache
    async fn get_value<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.client.clone();

        let value: Option<String> = conn
            .get(key)
            .await
            .context("Failed to get value from Redis")?;

        match value {
            Some(json) => {
                let entry: CacheEntry<T> = serde_json::from_str(&json)
                    .context("Failed to deserialize cache entry")?;
                Ok(Some(entry.data))
            }
            None => Ok(None),
        }
    }

    /// Generic set value in cache
    async fn set_value<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_seconds: Option<u64>,
    ) -> Result<()> {
        let entry = CacheEntry::new(value);
        let json = serde_json::to_string(&entry)
            .context("Failed to serialize cache entry")?;

        // Check size limit
        let size_mb = json.len() / (1024 * 1024);
        if size_mb > self.config.max_value_size_mb {
            warn!(
                "Cache value too large ({} MB > {} MB limit), skipping: {}",
                size_mb, self.config.max_value_size_mb, key
            );
            return Ok(());
        }

        let mut conn = self.client.clone();

        match ttl_seconds {
            Some(ttl) => {
                conn.set_ex::<_, _, ()>(key, json, ttl)
                    .await
                    .context("Failed to set value in Redis with TTL")?;
            }
            None => {
                conn.set::<_, _, ()>(key, json)
                    .await
                    .context("Failed to set value in Redis")?;
            }
        }

        Ok(())
    }
}

/// Redis cache statistics
#[derive(Debug, Clone)]
pub struct RedisStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

/// Stub implementation when Redis is not enabled
#[cfg(not(feature = "redis_cache"))]
pub struct RedisCache;

#[cfg(not(feature = "redis_cache"))]
impl RedisCache {
    pub async fn new(_config: RedisConfig) -> Result<Self> {
        anyhow::bail!("Redis cache support not compiled (missing redis_cache feature)")
    }

    pub async fn test_connection(&self) -> Result<()> {
        Ok(())
    }

    pub async fn get_query_results(&self, _query: &str) -> Result<Option<Vec<Uuid>>> {
        Ok(None)
    }

    pub async fn set_query_results(&self, _query: &str, _card_ids: &[Uuid]) -> Result<()> {
        Ok(())
    }

    pub async fn get_card(&self, _id: Uuid) -> Result<Option<Card>> {
        Ok(None)
    }

    pub async fn set_card(&self, _card: &Card) -> Result<()> {
        Ok(())
    }

    pub async fn get_cards(&self, _ids: &[Uuid]) -> Result<Vec<Card>> {
        Ok(Vec::new())
    }

    pub async fn set_cards(&self, _cards: &[Card]) -> Result<()> {
        Ok(())
    }

    pub async fn get_autocomplete(&self, _prefix: &str) -> Result<Option<Vec<String>>> {
        Ok(None)
    }

    pub async fn set_autocomplete(&self, _prefix: &str, _names: &[String]) -> Result<()> {
        Ok(())
    }

    pub async fn invalidate_all(&self) -> Result<()> {
        Ok(())
    }

    pub async fn get_stats(&self) -> Result<RedisStats> {
        Ok(RedisStats {
            hits: 0,
            misses: 0,
            hit_rate: 0.0,
        })
    }
}
