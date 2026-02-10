use anyhow::{Context, Result};
use chrono::DateTime;
use flate2::read::GzDecoder;
use serde::Deserialize;
use std::io::Read;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

use crate::config::ScryfallConfig;
use crate::db::Database;
use crate::metrics::{
    BULK_DATA_CARDS_IMPORTED, BULK_DATA_LAST_LOAD_TIMESTAMP, BULK_DATA_LOAD_DURATION_SECONDS,
};
use crate::models::card::Card;

const BULK_DATA_API: &str = "https://api.scryfall.com/bulk-data";
const BATCH_SIZE: usize = 500;
const MAX_RETRIES: u32 = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 1000; // Start with 1 second

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

#[derive(Clone)]
pub struct BulkLoader {
    db: Database,
    config: ScryfallConfig,
}

/// Retry a fallible async operation with exponential backoff
async fn retry_with_backoff<F, Fut, T, E>(
    operation: F,
    max_retries: u32,
    operation_name: &str,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    loop {
        attempt += 1;

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    info!(
                        "{} succeeded on attempt {}/{}",
                        operation_name, attempt, max_retries
                    );
                }
                return Ok(result);
            }
            Err(e) if attempt >= max_retries => {
                error!(
                    "{} failed after {} attempts: {}",
                    operation_name, max_retries, e
                );
                return Err(e);
            }
            Err(e) => {
                let delay = Duration::from_millis(INITIAL_RETRY_DELAY_MS * 2_u64.pow(attempt - 1));
                warn!(
                    "{} attempt {}/{} failed: {}. Retrying in {:?}...",
                    operation_name, attempt, max_retries, e, delay
                );
                tokio::time::sleep(delay).await;
            }
        }
    }
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
                info!(
                    "Bulk data is stale ({}h old), reload required",
                    hours_since_import
                );
                return Ok(true);
            } else {
                info!(
                    "Bulk data is fresh ({}h old), skipping reload",
                    hours_since_import
                );
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
        self.db
            .record_bulk_import(total_cards as i32, &source)
            .await?;

        let duration = start.elapsed();
        info!(
            "Bulk data import completed: {} cards imported in {:.2}s ({:.0} cards/sec)",
            total_cards,
            duration.as_secs_f64(),
            total_cards as f64 / duration.as_secs_f64()
        );

        // Record metrics
        BULK_DATA_LOAD_DURATION_SECONDS.set(duration.as_secs_f64());
        BULK_DATA_CARDS_IMPORTED.set(total_cards as i64);
        BULK_DATA_LAST_LOAD_TIMESTAMP.set(chrono::Utc::now().timestamp());

        Ok(())
    }

    /// Check if Scryfall's bulk data has been updated since our last import
    ///
    /// This enables "smart refresh" - only downloading when data actually changes
    pub async fn check_upstream_updated(&self) -> Result<bool> {
        // Get our last bulk data info if we have it
        let our_updated_at = self.db.get_last_bulk_import().await?;

        // Fetch Scryfall's current bulk data info
        let bulk_info = self.discover_bulk_data().await?;

        // Parse Scryfall's updated_at timestamp
        let their_updated_at = chrono::DateTime::parse_from_rfc3339(&bulk_info.updated_at)
            .context("Failed to parse Scryfall updated_at timestamp")?
            .naive_utc();

        // If we have previous import metadata, compare timestamps
        if let Some(our_time) = our_updated_at {
            let is_newer = their_updated_at > our_time;
            if is_newer {
                info!(
                    "Scryfall bulk data updated: ours={}, theirs={}",
                    our_time, their_updated_at
                );
            } else {
                debug!(
                    "Scryfall bulk data unchanged: ours={}, theirs={}",
                    our_time, their_updated_at
                );
            }
            Ok(is_newer)
        } else {
            // No metadata, assume we should update
            info!("No bulk import metadata found, assuming update needed");
            Ok(true)
        }
    }

    /// Discover the bulk data download URI
    async fn discover_bulk_data(&self) -> Result<BulkDataInfo> {
        let client = reqwest::Client::builder()
            .user_agent("scryfall-cache/0.1.0")
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;

        let bulk_type = self.config.bulk_data_type.clone();

        // Retry the bulk data discovery
        let response = retry_with_backoff(
            || async {
                client
                    .get(BULK_DATA_API)
                    .header("Accept", "application/json")
                    .send()
                    .await
                    .context("Failed to send request to bulk data API")
            },
            MAX_RETRIES,
            "Bulk data API request",
        )
        .await?;

        // Check response status
        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| String::from("Unable to read response body"));
            return Err(anyhow::anyhow!(
                "Scryfall bulk data API returned error status {}: {}. Check if API is available at {}",
                status,
                body,
                BULK_DATA_API
            ));
        }

        debug!("Bulk data API response status: {}", status);

        let response_body = response
            .text()
            .await
            .context("Failed to read bulk data API response body")?;

        debug!(
            "Bulk data API response body length: {} bytes",
            response_body.len()
        );

        let bulk_list: BulkDataList = serde_json::from_str(&response_body).context(format!(
            "Failed to parse bulk data list JSON. Response starts with: {}",
            &response_body[..response_body.len().min(200)]
        ))?;

        info!(
            "Discovered {} bulk data sets from Scryfall",
            bulk_list.data.len()
        );

        // Log available types for debugging
        let available_types: Vec<String> =
            bulk_list.data.iter().map(|d| d.bulk_type.clone()).collect();
        debug!("Available bulk data types: {:?}", available_types);

        // Find the requested bulk data type
        let bulk_info = bulk_list
            .data
            .into_iter()
            .find(|info| info.bulk_type == bulk_type)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Bulk data type '{}' not found. Available types: {:?}. Check SCRYFALL_BULK_DATA_TYPE environment variable.",
                    bulk_type,
                    available_types
                )
            })?;

        info!(
            "Selected bulk data: type='{}', size={:.2}MB, updated={}",
            bulk_info.bulk_type,
            bulk_info.size as f64 / 1_000_000.0,
            bulk_info.updated_at
        );

        Ok(bulk_info)
    }

    /// Download and import bulk data
    async fn download_and_import(&self, bulk_info: &BulkDataInfo) -> Result<usize> {
        let client = reqwest::Client::builder()
            .user_agent("scryfall-cache/0.1.0")
            .timeout(Duration::from_secs(600)) // 10 minutes for large downloads
            .build()
            .context("Failed to build HTTP client for bulk download")?;

        info!("Downloading bulk data from {}", bulk_info.download_uri);
        info!(
            "Expected size: {:.2} MB ({}  bytes)",
            bulk_info.size as f64 / 1_000_000.0,
            bulk_info.size
        );

        let download_uri = bulk_info.download_uri.clone();
        let expected_size = bulk_info.size;

        // Retry the download
        let bytes = retry_with_backoff(
            || async {
                let response = client
                    .get(&download_uri)
                    .send()
                    .await
                    .context("Failed to send download request")?;

                if !response.status().is_success() {
                    let status = response.status();
                    let body = response
                        .text()
                        .await
                        .unwrap_or_else(|_| String::from("Unable to read response"));
                    return Err(anyhow::anyhow!(
                        "Download failed with status {}: {}",
                        status,
                        body
                    ));
                }

                response
                    .bytes()
                    .await
                    .context("Failed to read response bytes")
            },
            MAX_RETRIES,
            "Bulk data download",
        )
        .await?;

        let actual_size = bytes.len() as i64;
        info!(
            "Download complete: {:.2} MB ({} bytes)",
            actual_size as f64 / 1_000_000.0,
            actual_size
        );

        // Verify download size is reasonable
        if actual_size == 0 {
            return Err(anyhow::anyhow!(
                "Downloaded bulk data is empty (0 bytes). Download may have failed."
            ));
        }

        // Warn if size differs significantly from expected
        let size_diff_pct =
            ((actual_size - expected_size) as f64 / expected_size as f64).abs() * 100.0;
        if size_diff_pct > 10.0 {
            warn!(
                "Downloaded size ({} bytes) differs from expected size ({} bytes) by {:.1}%",
                actual_size, expected_size, size_diff_pct
            );
        } else {
            debug!(
                "Download size matches expected (diff: {:.1}%)",
                size_diff_pct
            );
        }

        info!("Parsing bulk data...");

        // Try to parse as JSON directly first (in case reqwest auto-decompressed)
        let json_array: Vec<serde_json::Value> = match serde_json::from_slice::<
            Vec<serde_json::Value>,
        >(&bytes)
        {
            Ok(array) => {
                info!("Successfully parsed JSON directly (data was already decompressed)");
                debug!("Parsed {} top-level elements", array.len());
                array
            }
            Err(direct_parse_error) => {
                // If direct parsing fails, try decompressing first
                debug!(
                    "Direct JSON parsing failed: {}. Attempting gzip decompression...",
                    direct_parse_error
                );

                let mut decoder = GzDecoder::new(&bytes[..]);
                let mut decompressed = Vec::new();

                decoder.read_to_end(&mut decompressed).context(format!(
                    "Failed to decompress gzipped bulk data. This may not be gzip-encoded data. Original parse error was: {}",
                    direct_parse_error
                ))?;

                let decompressed_size = decompressed.len();
                info!(
                    "Decompression complete: {:.2} MB ({} bytes). Parsing JSON...",
                    decompressed_size as f64 / 1_000_000.0,
                    decompressed_size
                );

                serde_json::from_slice(&decompressed).context(format!(
                    "Failed to parse bulk data JSON after decompression. Decompressed size: {} bytes. Data preview: {}",
                    decompressed_size,
                    String::from_utf8_lossy(&decompressed[..decompressed_size.min(200)])
                ))?
            }
        };

        let total_cards = json_array.len();
        info!(
            "Parsed {} cards from JSON array, starting import...",
            total_cards
        );

        if total_cards == 0 {
            return Err(anyhow::anyhow!(
                "Bulk data JSON array is empty. Expected thousands of cards but got 0."
            ));
        }

        let mut imported = 0;
        let mut failed = 0;
        let mut batch = Vec::with_capacity(BATCH_SIZE);

        for (idx, card_json) in json_array.into_iter().enumerate() {
            match Card::from_scryfall_json(card_json.clone()) {
                Ok(card) => {
                    batch.push(card);

                    if batch.len() >= BATCH_SIZE {
                        self.db
                            .insert_cards_batch(&batch)
                            .await
                            .context(format!("Failed to insert batch at index {}", idx))?;
                        imported += batch.len();
                        batch.clear();

                        if imported % 5000 == 0 {
                            info!(
                                "Progress: {}/{} cards imported ({:.1}%), {} failed",
                                imported,
                                total_cards,
                                (imported as f64 / total_cards as f64) * 100.0,
                                failed
                            );
                        }
                    }
                }
                Err(e) => {
                    failed += 1;
                    if failed <= 10 {
                        // Log first 10 failures with details
                        warn!(
                            "Failed to parse card at index {}: {}. Card preview: {:?}",
                            idx,
                            e,
                            card_json
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                        );
                    } else if failed % 100 == 0 {
                        // Log every 100th failure after that
                        warn!("Failed card count: {}", failed);
                    }
                }
            }
        }

        // Import remaining cards
        if !batch.is_empty() {
            self.db
                .insert_cards_batch(&batch)
                .await
                .context("Failed to insert final batch")?;
            imported += batch.len();
        }

        info!(
            "Import complete: {}/{} cards imported successfully, {} failed to parse ({:.2}% success rate)",
            imported,
            total_cards,
            failed,
            (imported as f64 / total_cards as f64) * 100.0
        );

        // Verify we imported a reasonable number of cards
        if imported < 1000 {
            return Err(anyhow::anyhow!(
                "Import verification failed: Only {} cards imported. Expected at least 1000. This indicates a problem with the bulk data.",
                imported
            ));
        }

        if failed > total_cards / 10 {
            warn!(
                "Warning: High failure rate - {}/{} cards failed to parse ({}%). Data quality may be poor.",
                failed,
                total_cards,
                (failed as f64 / total_cards as f64) * 100.0
            );
        }

        Ok(imported)
    }

    /// Force reload bulk data regardless of cache status
    pub async fn force_load(&self) -> Result<()> {
        info!("Force loading bulk data...");
        self.load().await
    }
}
