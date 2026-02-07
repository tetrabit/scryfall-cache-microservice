use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::{debug, info};

use crate::config::ScryfallConfig;
use crate::models::card::Card;
use crate::scryfall::rate_limiter::RateLimiter;

const SCRYFALL_API_BASE: &str = "https://api.scryfall.com";

#[derive(Debug, Deserialize)]
struct SearchResponse {
    data: Vec<serde_json::Value>,
    has_more: bool,
    next_page: Option<String>,
}

/// Rate-limited Scryfall API client
#[derive(Clone)]
pub struct ScryfallClient {
    rate_limiter: RateLimiter,
    http_client: reqwest::Client,
}

impl ScryfallClient {
    pub fn new(config: &ScryfallConfig) -> Self {
        let rate_limiter = RateLimiter::new(config.rate_limit_per_second);

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
            // Wait for rate limit
            self.rate_limiter.acquire().await;

            // Make request
            let response = self
                .http_client
                .get(&url)
                .send()
                .await
                .context("Failed to send request to Scryfall")?;

            if !response.status().is_success() {
                let status = response.status();
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

        // Wait for rate limit
        self.rate_limiter.acquire().await;

        let endpoint = if fuzzy { "fuzzy" } else { "exact" };
        let url = format!(
            "{}/cards/named?{}={}",
            SCRYFALL_API_BASE,
            endpoint,
            urlencoding::encode(name)
        );

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to Scryfall")?;

        if response.status() == 404 {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
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

        let card = Card::from_scryfall_json(card_json)
            .context("Failed to convert Scryfall card")?;

        Ok(Some(card))
    }

    /// Get a card by Scryfall ID
    pub async fn get_card_by_id(&self, id: uuid::Uuid) -> Result<Option<Card>> {
        debug!("Fetching card by ID: {}", id);

        // Wait for rate limit
        self.rate_limiter.acquire().await;

        let url = format!("{}/cards/{}", SCRYFALL_API_BASE, id);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to Scryfall")?;

        if response.status() == 404 {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
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

        let card = Card::from_scryfall_json(card_json)
            .context("Failed to convert Scryfall card")?;

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
