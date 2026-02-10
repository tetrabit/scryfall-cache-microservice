use async_graphql::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::models::card::Card as DbCard;

/// GraphQL representation of a Magic: The Gathering card
#[derive(Debug, Clone)]
pub struct CardType {
    pub id: ID,
    pub oracle_id: Option<ID>,
    pub name: String,
    pub mana_cost: Option<String>,
    pub cmc: Option<f64>,
    pub type_line: Option<String>,
    pub oracle_text: Option<String>,
    pub colors: Option<Vec<String>>,
    pub color_identity: Option<Vec<String>>,
    pub set_code: Option<String>,
    pub set_name: Option<String>,
    pub collector_number: Option<String>,
    pub rarity: Option<String>,
    pub power: Option<String>,
    pub toughness: Option<String>,
    pub loyalty: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub released_at: Option<String>,
    pub prices_json: Option<JsonValue>,
    pub image_uris_json: Option<JsonValue>,
    pub legalities_json: Option<JsonValue>,
}

#[Object]
impl CardType {
    /// Unique card ID
    async fn id(&self) -> &ID {
        &self.id
    }

    /// Oracle ID (shared across reprints)
    async fn oracle_id(&self) -> &Option<ID> {
        &self.oracle_id
    }

    /// Card name
    async fn name(&self) -> &str {
        &self.name
    }

    /// Mana cost (e.g., "{2}{U}{U}")
    async fn mana_cost(&self) -> &Option<String> {
        &self.mana_cost
    }

    /// Converted mana cost
    async fn cmc(&self) -> Option<f64> {
        self.cmc
    }

    /// Type line (e.g., "Creature — Merfolk Wizard")
    async fn type_line(&self) -> &Option<String> {
        &self.type_line
    }

    /// Oracle rules text
    async fn oracle_text(&self) -> &Option<String> {
        &self.oracle_text
    }

    /// Card colors
    async fn colors(&self) -> &Option<Vec<String>> {
        &self.colors
    }

    /// Color identity (for Commander)
    async fn color_identity(&self) -> &Option<Vec<String>> {
        &self.color_identity
    }

    /// Set code (e.g., "LEA", "M21")
    async fn set_code(&self) -> &Option<String> {
        &self.set_code
    }

    /// Set name
    async fn set_name(&self) -> &Option<String> {
        &self.set_name
    }

    /// Collector number
    async fn collector_number(&self) -> &Option<String> {
        &self.collector_number
    }

    /// Rarity (common, uncommon, rare, mythic)
    async fn rarity(&self) -> &Option<String> {
        &self.rarity
    }

    /// Power (for creatures)
    async fn power(&self) -> &Option<String> {
        &self.power
    }

    /// Toughness (for creatures)
    async fn toughness(&self) -> &Option<String> {
        &self.toughness
    }

    /// Loyalty (for planeswalkers)
    async fn loyalty(&self) -> &Option<String> {
        &self.loyalty
    }

    /// Keywords (e.g., "Flying", "Haste")
    async fn keywords(&self) -> &Option<Vec<String>> {
        &self.keywords
    }

    /// Release date
    async fn released_at(&self) -> &Option<String> {
        &self.released_at
    }

    /// Get card prices (JSON string)
    async fn prices(&self) -> Option<String> {
        self.prices_json.as_ref().map(|v| v.to_string())
    }

    /// Get image URIs (JSON string)
    async fn image_uris(&self) -> Option<String> {
        self.image_uris_json.as_ref().map(|v| v.to_string())
    }

    /// Get legalities (JSON string)
    async fn legalities(&self) -> Option<String> {
        self.legalities_json.as_ref().map(|v| v.to_string())
    }

    /// Get USD price
    async fn usd_price(&self) -> Option<String> {
        self.prices_json
            .as_ref()
            .and_then(|v| v.get("usd"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Get USD foil price
    async fn usd_foil_price(&self) -> Option<String> {
        self.prices_json
            .as_ref()
            .and_then(|v| v.get("usd_foil"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

impl From<DbCard> for CardType {
    fn from(card: DbCard) -> Self {
        Self {
            id: ID(card.id.to_string()),
            oracle_id: card.oracle_id.map(|id| ID(id.to_string())),
            name: card.name,
            mana_cost: card.mana_cost,
            cmc: card.cmc,
            type_line: card.type_line,
            oracle_text: card.oracle_text,
            colors: card.colors,
            color_identity: card.color_identity,
            set_code: card.set_code,
            set_name: card.set_name,
            collector_number: card.collector_number,
            rarity: card.rarity,
            power: card.power,
            toughness: card.toughness,
            loyalty: card.loyalty,
            keywords: card.keywords,
            released_at: card.released_at.map(|d| d.to_string()),
            prices_json: card.prices,
            image_uris_json: card.image_uris,
            legalities_json: card.legalities,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, SimpleObject)]
pub struct CacheStatsType {
    /// Total number of cards in database
    pub total_cards: i64,

    /// Total number of cached query results
    pub total_cache_entries: i64,
}

/// Bulk data reload result
#[derive(Debug, Clone, SimpleObject)]
pub struct BulkDataReloadType {
    /// Whether the reload was successful
    pub success: bool,

    /// Status message
    pub message: String,
}

/// Input type for batch card queries
#[derive(Debug, Clone, InputObject)]
pub struct BatchCardInput {
    /// List of card IDs to fetch
    pub ids: Vec<ID>,

    /// Whether to fetch missing cards from Scryfall
    #[graphql(default = false)]
    pub fetch_missing: bool,
}

/// Result for batch card queries
#[derive(Debug, Clone, SimpleObject)]
pub struct BatchCardResult {
    /// Successfully fetched cards
    pub cards: Vec<CardType>,

    /// IDs that were not found
    pub missing_ids: Vec<ID>,
}
