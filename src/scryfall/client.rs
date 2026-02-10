use anyhow::{Context, Result};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError};
use crate::config::ScryfallConfig;
use crate::metrics::registry::{SCRYFALL_API_CALLS_TOTAL, SCRYFALL_API_ERRORS_TOTAL};
use crate::models::card::Card;
use crate::scryfall::rate_limiter::RateLimiter;

const SCRYFALL_API_BASE: &str = "https://api.scryfall.com";

#[derive(Debug, Deserialize)]
struct SearchResponse {
    data: Vec<serde_json::Value>,
    has_more: bool,
    next_page: Option<String>,
}

/// Rate-limited Scryfall API client with circuit breaker
#[derive(Clone)]
pub struct ScryfallClient {
    rate_limiter: RateLimiter,
    http_client: reqwest::Client,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl ScryfallClient {
    pub fn new(config: &ScryfallConfig) -> Self {
        let rate_limiter = RateLimiter::new(config.rate_limit_per_second);
        let cb_config = CircuitBreakerConfig::from_env();
        let circuit_breaker = Arc::new(CircuitBreaker::new("scryfall_api", cb_config));

        // Build HTTP client with required headers
        let http_client = reqwest::Client::builder()
            .user_agent("scryfall-cache/0.1.0")
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::ACCEPT,
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
                headers
            })
            .build()
            .expect("Failed to build HTTP client");

        info!(
            "Initialized Scryfall client with rate limit: {} req/sec",
            config.rate_limit_per_second
        );

        Self {
            rate_limiter,
            http_client,
            circuit_breaker,
        }
    }

    /// Make an HTTP request through the circuit breaker
    async fn make_request(&self, endpoint: &'static str, url: String) -> Result<reqwest::Response> {
        SCRYFALL_API_CALLS_TOTAL.with_label_values(&[endpoint]).inc();

        // Wait for rate limit first
        self.rate_limiter.acquire().await;

        // Execute through circuit breaker
        let client = self.http_client.clone();
        match self
            .circuit_breaker
            .call(async move {
                client
                    .get(&url)
                    .send()
                    .await
                    .context("Failed to send request to Scryfall")
            })
            .await
        {
            Ok(response) => Ok(response),
            Err(CircuitBreakerError::Open) => {
                warn!("Circuit breaker open, request rejected");
                Err(anyhow::anyhow!(
                    "Circuit breaker is open - Scryfall API unavailable"
                ))
            }
            Err(CircuitBreakerError::Inner(e)) => Err(e),
        }
    }

    /// Search for cards using Scryfall query syntax
    pub async fn search_cards(&self, query: &str) -> Result<Vec<Card>> {
        debug!("Searching Scryfall for: {}", query);

        let mut cards = Vec::new();
        let mut next_page: Option<String> = Some(format!(
            "{}/cards/search?q={}",
            SCRYFALL_API_BASE,
            urlencoding::encode(query)
        ));

        while let Some(url) = next_page {
            // Make request through circuit breaker
            let response = self.make_request("cards_search", url.clone()).await?;

            if !response.status().is_success() {
                let status = response.status();
                SCRYFALL_API_ERRORS_TOTAL
                    .with_label_values(&[&status.as_u16().to_string()])
                    .inc();
                let error_text = response.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!(
                    "Scryfall API error: {} - {}",
                    status,
                    error_text
                ));
            }

            let search_response: SearchResponse = response
                .json()
                .await
                .context("Failed to parse Scryfall response")?;

            // Convert each card
            for card_json in search_response.data {
                match Card::from_scryfall_json(card_json) {
                    Ok(card) => cards.push(card),
                    Err(e) => {
                        debug!("Failed to convert Scryfall card: {}", e);
                    }
                }
            }

            // Check if there are more pages
            if search_response.has_more {
                next_page = search_response.next_page;
            } else {
                next_page = None;
            }
        }

        info!(
            "Found {} cards from Scryfall for query: {}",
            cards.len(),
            query
        );
        Ok(cards)
    }

    /// Get a card by exact name
    pub async fn get_card_by_name(&self, name: &str, fuzzy: bool) -> Result<Option<Card>> {
        debug!("Fetching card by name: {} (fuzzy={})", name, fuzzy);

        let endpoint = if fuzzy { "fuzzy" } else { "exact" };
        let url = format!(
            "{}/cards/named?{}={}",
            SCRYFALL_API_BASE,
            endpoint,
            urlencoding::encode(name)
        );

        let response = self.make_request("cards_named", url).await?;

        if response.status() == 404 {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            SCRYFALL_API_ERRORS_TOTAL
                .with_label_values(&[&status.as_u16().to_string()])
                .inc();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Scryfall API error: {} - {}",
                status,
                error_text
            ));
        }

        let card_json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Scryfall response")?;

        let card =
            Card::from_scryfall_json(card_json).context("Failed to convert Scryfall card")?;

        Ok(Some(card))
    }

    /// Get a card by Scryfall ID
    pub async fn get_card_by_id(&self, id: uuid::Uuid) -> Result<Option<Card>> {
        debug!("Fetching card by ID: {}", id);

        let url = format!("{}/cards/{}", SCRYFALL_API_BASE, id);

        let response = self.make_request("cards_id", url).await?;

        if response.status() == 404 {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            SCRYFALL_API_ERRORS_TOTAL
                .with_label_values(&[&status.as_u16().to_string()])
                .inc();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Scryfall API error: {} - {}",
                status,
                error_text
            ));
        }

        let card_json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Scryfall response")?;

        let card =
            Card::from_scryfall_json(card_json).context("Failed to convert Scryfall card")?;

        Ok(Some(card))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_initialization() {
        let config = ScryfallConfig {
            rate_limit_per_second: 10,
            bulk_data_type: "default_cards".to_string(),
            cache_ttl_hours: 24,
        };

        let client = ScryfallClient::new(&config);
        assert_eq!(client.rate_limiter.requests_per_second(), 10);
    }
}
