use async_graphql::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::cache::manager::CacheManager;
use crate::graphql::types::*;
use crate::scryfall::bulk_loader::BulkLoader;

/// GraphQL Query root
pub struct Query;

#[Object]
impl Query {
    /// Get a card by its ID
    async fn card(&self, ctx: &Context<'_>, id: ID) -> Result<Option<CardType>> {
        let cache_manager = ctx.data::<Arc<CacheManager>>()?;

        let card_id = Uuid::parse_str(&id.0)
            .map_err(|e| Error::new(format!("Invalid UUID: {}", e)))?;

        let card = cache_manager
            .get_card(card_id)
            .await
            .map_err(|e| Error::new(format!("Failed to fetch card: {}", e)))?;

        Ok(card.map(CardType::from))
    }

    /// Get a card by name (fuzzy or exact match)
    async fn card_by_name(
        &self,
        ctx: &Context<'_>,
        name: String,
        #[graphql(default = true)] fuzzy: bool,
    ) -> Result<Option<CardType>> {
        let cache_manager = ctx.data::<Arc<CacheManager>>()?;

        let card = cache_manager
            .search_by_name(&name, fuzzy)
            .await
            .map_err(|e| Error::new(format!("Failed to search by name: {}", e)))?;

        Ok(card.map(CardType::from))
    }

    /// Search for cards using Scryfall query syntax
    async fn search_cards(
        &self,
        ctx: &Context<'_>,
        query: String,
        #[graphql(default = 100)] limit: i64,
    ) -> Result<Vec<CardType>> {
        let cache_manager = ctx.data::<Arc<CacheManager>>()?;

        let cards = cache_manager
            .search(&query, Some(limit))
            .await
            .map_err(|e| Error::new(format!("Failed to search cards: {}", e)))?;

        Ok(cards.into_iter().map(CardType::from).collect())
    }

    /// Autocomplete card names by prefix
    async fn autocomplete(&self, ctx: &Context<'_>, prefix: String) -> Result<Vec<String>> {
        let cache_manager = ctx.data::<Arc<CacheManager>>()?;

        let names = cache_manager
            .autocomplete(&prefix)
            .await
            .map_err(|e| Error::new(format!("Failed to autocomplete: {}", e)))?;

        Ok(names)
    }

    /// Get multiple cards by IDs in one request
    async fn cards_batch(
        &self,
        ctx: &Context<'_>,
        input: BatchCardInput,
    ) -> Result<BatchCardResult> {
        let cache_manager = ctx.data::<Arc<CacheManager>>()?;

        // Parse UUIDs
        let ids: Result<Vec<Uuid>, _> = input
            .ids
            .iter()
            .map(|id| {
                Uuid::parse_str(&id.0)
                    .map_err(|e| Error::new(format!("Invalid UUID '{}': {}", id.0, e)))
            })
            .collect();

        let ids = ids?;

        let (cards, missing_ids) = cache_manager
            .get_cards_batch(&ids, input.fetch_missing)
            .await
            .map_err(|e| Error::new(format!("Failed to fetch batch cards: {}", e)))?;

        Ok(BatchCardResult {
            cards: cards.into_iter().map(CardType::from).collect(),
            missing_ids: missing_ids.into_iter().map(|id| ID(id.to_string())).collect(),
        })
    }

    /// Get cache statistics
    async fn stats(&self, ctx: &Context<'_>) -> Result<CacheStatsType> {
        let cache_manager = ctx.data::<Arc<CacheManager>>()?;

        let stats = cache_manager
            .get_stats()
            .await
            .map_err(|e| Error::new(format!("Failed to get stats: {}", e)))?;

        Ok(CacheStatsType {
            total_cards: stats.total_cards,
            total_cache_entries: stats.total_cache_entries,
        })
    }
}

/// GraphQL Mutation root
pub struct Mutation;

#[Object]
impl Mutation {
    /// Manually trigger a bulk data reload from Scryfall
    async fn reload_bulk_data(&self, ctx: &Context<'_>) -> Result<BulkDataReloadType> {
        let bulk_loader = ctx.data::<Arc<BulkLoader>>()?;

        match bulk_loader.load().await {
            Ok(_) => Ok(BulkDataReloadType {
                success: true,
                message: "Bulk data reloaded successfully".to_string(),
            }),
            Err(e) => Ok(BulkDataReloadType {
                success: false,
                message: format!("Failed to reload bulk data: {}", e),
            }),
        }
    }
}

/// GraphQL schema type
pub type GraphQLSchema = Schema<Query, Mutation, EmptySubscription>;

/// Create a new GraphQL schema with the given cache manager
pub fn create_schema(
    cache_manager: Arc<CacheManager>,
    bulk_loader: Arc<BulkLoader>,
) -> GraphQLSchema {
    Schema::build(Query, Mutation, EmptySubscription)
        .data(cache_manager)
        .data(bulk_loader)
        .finish()
}
