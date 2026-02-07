use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use serde::Deserialize;
use sqlx::PgPool;
use std::time::Instant;
use tracing::{info, warn};

use crate::config::ScryfallConfig;
use crate::db::queries::{insert_cards_batch, record_bulk_import};
use crate::db::schema::{get_last_bulk_import, check_bulk_data_loaded};
use crate::models::card::Card;

const BULK_DATA_API: &str = "https://api.scryfall.com/bulk-data";
const BATCH_SIZE: usize = 500;

#[derive(Debug, Deserialize)]
struct BulkDataList {
    data: Vec<BulkDataInfo>,
}

#[derive(Debug, Deserialize)]
struct BulkDataInfo {
    #[serde(rename = "type")]
    bulk_type: String,
    download_uri: String,
    updated_at: String,
    size: i64,
}

pub struct BulkLoader {
    pool: PgPool,
    config: ScryfallConfig,
}

impl BulkLoader {
    pub fn new(pool: PgPool, config: ScryfallConfig) -> Self {
        Self { pool, config }
    }

    /// Check if bulk data should be loaded
    pub async fn should_load(&self) -> Result<bool> {
        // Check if database has any cards
        let has_cards = check_bulk_data_loaded(&self.pool).await?;

        if !has_cards {
            info!("No cards in database, bulk data load required");
            return Ok(true);
        }

        // Check when last import was done
        let last_import = get_last_bulk_import(&self.pool).await?;

        if let Some(last_import) = last_import {
            let hours_since_import = chrono::Utc::now()
                .naive_utc()
                .signed_duration_since(last_import)
                .num_hours();

            if hours_since_import >= self.config.cache_ttl_hours as i64 {
                info!("Bulk data is stale ({}h old), reload required", hours_since_import);
                return Ok(true);
            } else {
                info!("Bulk data is fresh ({}h old), skipping reload", hours_since_import);
                return Ok(false);
            }
        }

        // If we have cards but no metadata, don't reload
        info!("Cards exist but no import metadata found, skipping reload");
        Ok(false)
    }

    /// Load bulk data from Scryfall
    pub async fn load(&self) -> Result<()> {
        let start = Instant::now();
        info!("Starting bulk data import...");

        // Discover bulk data info
        let bulk_info = self.discover_bulk_data().await?;
        info!(
            "Found bulk data: type={}, size={}MB",
            bulk_info.bulk_type,
            bulk_info.size / 1_000_000
        );

        // Download and process
        let total_cards = self.download_and_import(&bulk_info).await?;

        // Record the import
        let updated_at = NaiveDateTime::parse_from_str(&bulk_info.updated_at, "%Y-%m-%dT%H:%M:%S%.fZ")
            .context("Failed to parse updated_at timestamp")?;

        record_bulk_import(
            &self.pool,
            &bulk_info.bulk_type,
            &bulk_info.download_uri,
            updated_at,
            total_cards as i32,
            bulk_info.size,
        )
        .await?;

        let duration = start.elapsed();
        info!(
            "Bulk data import completed: {} cards imported in {:.2}s ({:.0} cards/sec)",
            total_cards,
            duration.as_secs_f64(),
            total_cards as f64 / duration.as_secs_f64()
        );

        Ok(())
    }

    /// Discover the bulk data download URI
    async fn discover_bulk_data(&self) -> Result<BulkDataInfo> {
        let client = reqwest::Client::new();
        let response = client
            .get(BULK_DATA_API)
            .send()
            .await
            .context("Failed to fetch bulk data list")?;

        let bulk_list: BulkDataList = response
            .json()
            .await
            .context("Failed to parse bulk data list")?;

        // Find the requested bulk data type
        let bulk_info = bulk_list
            .data
            .into_iter()
            .find(|info| info.bulk_type == self.config.bulk_data_type)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Bulk data type '{}' not found",
                    self.config.bulk_data_type
                )
            })?;

        Ok(bulk_info)
    }

    /// Download and import bulk data
    async fn download_and_import(&self, bulk_info: &BulkDataInfo) -> Result<usize> {
        let client = reqwest::Client::new();
        let response = client
            .get(&bulk_info.download_uri)
            .send()
            .await
            .context("Failed to download bulk data")?;

        info!("Downloading bulk data from {}", bulk_info.download_uri);

        // Get the response body as bytes stream
        let bytes = response
            .bytes()
            .await
            .context("Failed to read bulk data")?;

        info!("Download complete, parsing and importing cards...");

        // Parse JSON array
        let json_array: Vec<serde_json::Value> = serde_json::from_slice(&bytes)
            .context("Failed to parse bulk data JSON")?;

        let total_cards = json_array.len();
        let mut imported = 0;
        let mut batch = Vec::with_capacity(BATCH_SIZE);

        for (idx, card_json) in json_array.into_iter().enumerate() {
            match Card::from_scryfall_json(card_json) {
                Ok(card) => {
                    batch.push(card);

                    if batch.len() >= BATCH_SIZE {
                        insert_cards_batch(&self.pool, &batch).await?;
                        imported += batch.len();
                        batch.clear();

                        if imported % 5000 == 0 {
                            info!("Imported {}/{} cards ({:.1}%)", imported, total_cards, (imported as f64 / total_cards as f64) * 100.0);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to parse card at index {}: {}", idx, e);
                }
            }
        }

        // Import remaining cards
        if !batch.is_empty() {
            insert_cards_batch(&self.pool, &batch).await?;
            imported += batch.len();
        }

        info!("Import complete: {}/{} cards imported", imported, total_cards);

        Ok(imported)
    }

    /// Force reload bulk data regardless of cache status
    pub async fn force_load(&self) -> Result<()> {
        info!("Force loading bulk data...");
        self.load().await
    }
}
