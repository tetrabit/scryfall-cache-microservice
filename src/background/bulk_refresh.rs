use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info};

use crate::scryfall::bulk_loader::BulkLoader;

/// Configuration for bulk data refresh job
#[derive(Debug, Clone)]
pub struct BulkRefreshConfig {
    /// Whether background refresh is enabled
    pub enabled: bool,
    /// Interval between refresh checks (hours)
    pub check_interval_hours: u64,
}

impl Default for BulkRefreshConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_hours: 720, // 30 days (monthly)
        }
    }
}

impl BulkRefreshConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("BULK_REFRESH_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            check_interval_hours: std::env::var("BULK_REFRESH_INTERVAL_HOURS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(720), // Default: 30 days
        }
    }
}

/// Start background bulk data refresh job
///
/// This spawns a background task that periodically checks if Scryfall's bulk data
/// has been updated and refreshes the local cache if needed.
///
/// # Smart Refresh Strategy
/// 1. Check Scryfall's `updated_at` timestamp
/// 2. Only download if upstream data actually changed
/// 3. Fall back to time-based refresh if check fails
///
/// # Arguments
/// * `bulk_loader` - Shared BulkLoader instance
/// * `config` - Refresh configuration (interval, etc.)
///
/// # Returns
/// tokio::task::JoinHandle that can be awaited or aborted
pub fn start_bulk_refresh_job(
    bulk_loader: Arc<BulkLoader>,
    config: BulkRefreshConfig,
) -> tokio::task::JoinHandle<()> {
    if !config.enabled {
        info!("Bulk data refresh job is disabled");
        return tokio::spawn(async {});
    }

    let interval_duration = Duration::from_secs(config.check_interval_hours * 3600);
    info!(
        "Starting bulk data refresh job: checking every {} hours ({} days)",
        config.check_interval_hours,
        config.check_interval_hours / 24
    );

    tokio::spawn(async move {
        let mut ticker = interval(interval_duration);

        // Skip the first tick (happens immediately)
        ticker.tick().await;

        loop {
            ticker.tick().await;

            info!("Scheduled bulk data refresh check (interval: {} hours)...", config.check_interval_hours);

            // First, check if Scryfall's data has actually updated
            match bulk_loader.check_upstream_updated().await {
                Ok(true) => {
                    info!("Scryfall bulk data has been updated upstream, downloading...");
                    match bulk_loader.load().await {
                        Ok(()) => {
                            info!("Scheduled bulk data refresh completed successfully");
                        }
                        Err(e) => {
                            error!("Scheduled bulk data refresh failed: {}", e);
                        }
                    }
                }
                Ok(false) => {
                    info!("Scryfall bulk data unchanged since last import, skipping download");
                }
                Err(e) => {
                    error!("Failed to check upstream bulk data status: {}", e);
                    info!("Falling back to time-based refresh check...");

                    // Fall back to time-based check
                    match bulk_loader.should_load().await {
                        Ok(true) => {
                            info!("Time-based refresh triggered (fallback mode)");
                            if let Err(e) = bulk_loader.load().await {
                                error!("Fallback bulk data refresh failed: {}", e);
                            }
                        }
                        Ok(false) => {
                            info!("Time-based check: bulk data is still fresh");
                        }
                        Err(e) => {
                            error!("Failed to check if bulk data should load: {}", e);
                        }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BulkRefreshConfig::default();
        assert!(config.enabled);
        assert_eq!(config.check_interval_hours, 720); // 30 days
    }

    #[test]
    fn test_monthly_interval() {
        let config = BulkRefreshConfig {
            enabled: true,
            check_interval_hours: 720,
        };
        assert_eq!(config.check_interval_hours / 24, 30); // 30 days
    }
}
