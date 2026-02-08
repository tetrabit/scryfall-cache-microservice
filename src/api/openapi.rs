use utoipa::OpenApi;

use crate::api::handlers::{
    CardListResponse, CardResponse, NamedParams, PaginatedCardData, ReloadResponse, SearchParams,
    StatsResponse,
};
use crate::cache::manager::CacheStats;
use crate::models::card::Card;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Scryfall Cache Microservice",
        version = "0.1.0",
        description = "A high-performance caching microservice for Scryfall Magic: The Gathering card data. Provides fast search and retrieval of card information with built-in caching and bulk data loading.",
        contact(
            name = "Scryfall Cache API",
        )
    ),
    paths(
        crate::api::handlers::health,
        crate::api::handlers::search_cards,
        crate::api::handlers::get_card_by_name,
        crate::api::handlers::get_card,
        crate::api::handlers::get_stats,
        crate::api::handlers::admin_reload,
    ),
    components(
        schemas(
            Card,
            CardResponse,
            CardListResponse,
            PaginatedCardData,
            StatsResponse,
            ReloadResponse,
            CacheStats,
            SearchParams,
            NamedParams,
        )
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "cards", description = "Card search and retrieval endpoints"),
        (name = "statistics", description = "Cache statistics and metrics"),
        (name = "admin", description = "Administrative endpoints"),
    )
)]
pub struct ApiDoc;
