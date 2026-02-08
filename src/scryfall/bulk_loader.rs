use anyhow::{Context, Result};
use chrono::DateTime;
use flate2::read::GzDecoder;
use serde::Deserialize;
use std::io::Read;
use std::time::Instant;
use tracing::{info, warn};

use crate::config::ScryfallConfig;
use crate::db::Database;
use crate::models::card::Card;

const BULK_DATA_API: &str = "https://api.scryfall.com/bulk-data";
const BATCH_SIZE: usize = 500;

#[derive(Debug, Deserialize)]
struct BulkDataList {
    object: String,
    has_more: bool,
    data: Vec<BulkDataInfo>,
}

#[derive(Debug, Deserialize)]
struct BulkDataInfo {
    object: String,
    id: String,
    #[serde(rename = "type")]
    bulk_type: String,
    updated_at: String,
    uri: String,
    name: String,
    description: String,
    size: i64,
    download_uri: String,
    content_type: String,
    content_encoding: String,
}

pub struct BulkLoader {
    db: Database,
    config: ScryfallConfig,
}

impl BulkLoader {
    pub fn new(db: Database, config: ScryfallConfig) -> Self {
        Self { db, config }
    }

    /// Check if bulk data should be loaded
    pub async fn should_load(&self) -> Result<bool> {
        // Check if database has any cards
        let has_cards = self.db.check_bulk_data_loaded().await?;

        if !has_cards {
            info!("No cards in database, bulk data load required");
            return Ok(true);
        }

        // Check when last import was done
        let last_import = self.db.get_last_bulk_import().await?;

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
        let _updated_at = DateTime::parse_from_rfc3339(&bulk_info.updated_at)
            .context("Failed to parse updated_at timestamp")?
            .naive_utc();

        // Get the bulk type for the source field
        let source = bulk_info.download_uri.clone();
        self.db.record_bulk_import(
            total_cards as i32,
            &source,
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
        let client = reqwest::Client::builder()
            .user_agent("scryfall-cache/0.1.0")
            .build()
            .context("Failed to build HTTP client")?;

        let response = client
            .get(BULK_DATA_API)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to fetch bulk data list")?;

        // Check response status
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| String::from("Unable to read response body"));
            return Err(anyhow::anyhow!(
                "Scryfall API returned error {}: {}",
                status,
                body
            ));
        }

        let bulk_list: BulkDataList = response
            .json()
            .await
            .context("Failed to parse bulk data list")?;

        info!("Discovered {} bulk data sets", bulk_list.data.len());

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
        let client = reqwest::Client::builder()
            .user_agent("scryfall-cache/0.1.0")
            .build()
            .context("Failed to build HTTP client")?;

        info!("Downloading bulk data from {}", bulk_info.download_uri);
        info!("Expected size: {} MB", bulk_info.size / 1_000_000);

        let response = client
            .get(&bulk_info.download_uri)
            .send()
            .await
            .context("Failed to download bulk data")?;

        // Get the response body as bytes
        let bytes = response
            .bytes()
            .await
            .context("Failed to read bulk data")?;

        info!("Download complete ({} MB), parsing...", bytes.len() / 1_000_000);

        // Try to parse as JSON directly first (in case reqwest auto-decompressed)
        let json_array: Vec<serde_json::Value> = match serde_json::from_slice(&bytes) {
            Ok(array) => {
                info!("Successfully parsed JSON directly (data was already decompressed)");
                array
            }
            Err(_) => {
                // If direct parsing fails, try decompressing first
                info!("Direct JSON parsing failed, attempting gzip decompression...");
                let mut decoder = GzDecoder::new(&bytes[..]);
                let mut decompressed = Vec::new();
                decoder
                    .read_to_end(&mut decompressed)
                    .context("Failed to decompress gzipped bulk data")?;

                info!("Decompression complete ({} MB), parsing JSON...", decompressed.len() / 1_000_000);

                serde_json::from_slice(&decompressed)
                    .context("Failed to parse bulk data JSON after decompression")?
            }
        };

        let total_cards = json_array.len();
        info!("Parsed {} cards, starting import...", total_cards);

        let mut imported = 0;
        let mut batch = Vec::with_capacity(BATCH_SIZE);

        for (idx, card_json) in json_array.into_iter().enumerate() {
            match Card::from_scryfall_json(card_json) {
                Ok(card) => {
                    batch.push(card);

                    if batch.len() >= BATCH_SIZE {
                        self.db.insert_cards_batch(&batch).await?;
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
            self.db.insert_cards_batch(&batch).await?;
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
