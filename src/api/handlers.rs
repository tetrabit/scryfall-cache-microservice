use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::cache::manager::{CacheManager, CacheStats};
use crate::errors::ErrorResponse;
use crate::models::card::Card;
use crate::query::{QueryLimits, QueryParser, QueryValidator};
use crate::scryfall::bulk_loader::BulkLoader;

lazy_static::lazy_static! {
    static ref START_TIME: Instant = Instant::now();
}

pub type AppState = Arc<AppStateInner>;

pub struct AppStateInner {
    pub cache_manager: CacheManager,
    pub bulk_loader: BulkLoader,
    pub query_validator: QueryValidator,
}

/// Generic API response wrapper
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiResponse<T> {
    /// Whether the request was successful
    pub success: bool,
    /// Response data (present if success is true)
    pub data: Option<T>,
    /// Error details (present if success is false)
    pub error: Option<crate::errors::response::ErrorDetail>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
}

/// Search query parameters
#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct SearchParams {
    /// Scryfall search query (e.g., "c:red cmc:1" or "Sol Ring")
    pub q: String,
    /// Maximum number of results to return (default: unlimited)
    pub limit: Option<i64>,
    /// Page number for pagination (starts at 1)
    pub page: Option<usize>,
    /// Number of results per page (default: 100, max: 1000)
    pub page_size: Option<usize>,
}

/// Paginated response wrapper
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedResponse<T> {
    /// Array of results for the current page
    pub data: Vec<T>,
    /// Total number of results across all pages
    pub total: usize,
    /// Current page number
    pub page: usize,
    /// Number of results per page
    pub page_size: usize,
    /// Total number of pages
    pub total_pages: usize,
    /// Whether there are more pages available
    pub has_more: bool,
}

/// Named card lookup parameters
#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct NamedParams {
    /// Fuzzy card name search (e.g., "light bolt" matches "Lightning Bolt")
    pub fuzzy: Option<String>,
    /// Exact card name search (case-insensitive)
    pub exact: Option<String>,
}

/// Autocomplete query parameters
#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct AutocompleteParams {
    /// Card name prefix to search for (e.g., "light" matches "Lightning Bolt")
    pub q: String,
}

// Concrete response types for OpenAPI generation
/// Card response
#[derive(Debug, Serialize, ToSchema)]
pub struct CardResponse {
    /// Whether the request was successful
    pub success: bool,
    /// Response data (present if success is true)
    pub data: Option<Card>,
    /// Error details (present if success is false)
    pub error: Option<crate::errors::response::ErrorDetail>,
}

/// Paginated card list response
#[derive(Debug, Serialize, ToSchema)]
pub struct CardListResponse {
    /// Whether the request was successful
    pub success: bool,
    /// Response data (present if success is true)
    pub data: Option<PaginatedCardData>,
    /// Error details (present if success is false)
    pub error: Option<crate::errors::response::ErrorDetail>,
}

/// Paginated card data
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedCardData {
    /// Array of results for the current page
    pub data: Vec<Card>,
    /// Total number of results across all pages
    pub total: usize,
    /// Current page number
    pub page: usize,
    /// Number of results per page
    pub page_size: usize,
    /// Total number of pages
    pub total_pages: usize,
    /// Whether there are more pages available
    pub has_more: bool,
}

/// Cache statistics response
#[derive(Debug, Serialize, ToSchema)]
pub struct StatsResponse {
    /// Whether the request was successful
    pub success: bool,
    /// Response data (present if success is true)
    pub data: Option<CacheStats>,
    /// Error details (present if success is false)
    pub error: Option<crate::errors::response::ErrorDetail>,
}

/// Reload response
#[derive(Debug, Serialize, ToSchema)]
pub struct ReloadResponse {
    /// Whether the request was successful
    pub success: bool,
    /// Response data (present if success is true)
    pub data: Option<String>,
    /// Error details (present if success is false)
    pub error: Option<crate::errors::response::ErrorDetail>,
}

/// Autocomplete response (Scryfall catalog format)
#[derive(Debug, Serialize, ToSchema)]
pub struct AutocompleteResponse {
    /// Object type (always "catalog")
    pub object: String,
    /// Array of card name suggestions
    pub data: Vec<String>,
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = serde_json::Value)
    )
)]
pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "scryfall-cache",
        "version": env!("CARGO_PKG_VERSION"),
        "build": {
            "version": env!("CARGO_PKG_VERSION"),
            "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        },
        "environment": {
            "rust_version": env!("CARGO_PKG_RUST_VERSION", "unknown"),
            "database": std::env::var("DATABASE_URL")
                .map(|url| if url.starts_with("postgres") { "postgresql" } else { "sqlite" })
                .unwrap_or("unknown"),
        },
        "uptime_seconds": START_TIME.elapsed().as_secs(),
    }))
}

/// Search for cards
#[utoipa::path(
    get,
    path = "/cards/search",
    tag = "cards",
    params(SearchParams),
    responses(
        (status = 200, description = "Search results", body = CardListResponse),
        (status = 500, description = "Internal server error", body = CardListResponse)
    )
)]
pub async fn search_cards(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    info!("Search request: query='{}', limit={:?}, page={:?}, page_size={:?}",
        params.q, params.limit, params.page, params.page_size);

    // Validate query string
    if let Err(e) = state.query_validator.validate_query_string(&params.q) {
        return ErrorResponse::validation_error(e.to_string()).into_response();
    }

    // Parse and validate query AST
    match QueryParser::parse(&params.q) {
        Ok(ast) => {
            if let Err(e) = state.query_validator.validate_ast(&ast) {
                return ErrorResponse::validation_error(e.to_string()).into_response();
            }
        }
        Err(e) => {
            return ErrorResponse::invalid_query(format!("Query parse error: {}", e)).into_response();
        }
    }

    // Use pagination parameters
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(100).min(1000).max(1);

    // Use the new paginated search which is much faster
    match state.cache_manager.search_paginated(&params.q, page, page_size).await {
        Ok((cards, total)) => {
            let total_pages = (total + page_size - 1) / page_size;
            let has_more = page < total_pages;

            info!("Search returned {} cards (page {}/{}), {} total matches",
                cards.len(), page, total_pages, total);

            let response = PaginatedResponse {
                data: cards,
                total,
                page,
                page_size,
                total_pages,
                has_more,
            };

            (StatusCode::OK, Json(ApiResponse::success(response))).into_response()
        }
        Err(e) => {
            error!("Search failed: {}", e);
            
            // Map error type to appropriate error code
            let error_message = e.to_string();
            if error_message.contains("database") || error_message.contains("connection") || error_message.contains("pool") {
                ErrorResponse::database_error(format!("Database error during search: {}", e)).into_response()
            } else {
                ErrorResponse::invalid_query(format!("Search failed: {}", e)).into_response()
            }
        }
    }
}

/// Get a specific card by ID
#[utoipa::path(
    get,
    path = "/cards/{id}",
    tag = "cards",
    params(
        ("id" = Uuid, Path, description = "Card UUID")
    ),
    responses(
        (status = 200, description = "Card found", body = CardResponse),
        (status = 404, description = "Card not found", body = CardResponse),
        (status = 500, description = "Internal server error", body = CardResponse)
    )
)]
pub async fn get_card(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    info!("Get card request: id={}", id);

    match state.cache_manager.get_card(id).await {
        Ok(Some(card)) => {
            info!("Found card: {}", card.name);
            (StatusCode::OK, Json(ApiResponse::success(card))).into_response()
        }
        Ok(None) => {
            info!("Card not found: {}", id);
            ErrorResponse::card_not_found(id.to_string()).into_response()
        }
        Err(e) => {
            error!("Get card failed: {}", e);
            ErrorResponse::database_error(format!("Failed to fetch card: {}", e)).into_response()
        }
    }
}

/// Get a card by name (fuzzy or exact)
#[utoipa::path(
    get,
    path = "/cards/named",
    tag = "cards",
    params(NamedParams),
    responses(
        (status = 200, description = "Card found", body = CardResponse),
        (status = 400, description = "Bad request - must provide fuzzy or exact parameter", body = CardResponse),
        (status = 404, description = "Card not found", body = CardResponse),
        (status = 500, description = "Internal server error", body = CardResponse)
    )
)]
pub async fn get_card_by_name(
    State(state): State<AppState>,
    Query(params): Query<NamedParams>,
) -> impl IntoResponse {
    let (name, fuzzy) = if let Some(name) = params.fuzzy {
        (name, true)
    } else if let Some(name) = params.exact {
        (name, false)
    } else {
        return ErrorResponse::validation_error(
            "Must provide either 'fuzzy' or 'exact' parameter"
        ).into_response();
    };

    info!("Get card by name: name='{}', fuzzy={}", name, fuzzy);

    match state.cache_manager.search_by_name(&name, fuzzy).await {
        Ok(Some(card)) => {
            info!("Found card: {}", card.name);
            (StatusCode::OK, Json(ApiResponse::success(card))).into_response()
        }
        Ok(None) => {
            info!("Card not found: {}", name);
            ErrorResponse::card_not_found(name).into_response()
        }
        Err(e) => {
            error!("Get card by name failed: {}", e);
            
            // Map error type to appropriate error code
            let error_message = e.to_string();
            if error_message.contains("database") || error_message.contains("connection") || error_message.contains("pool") {
                ErrorResponse::database_error(format!("Database error: {}", e)).into_response()
            } else if error_message.contains("scryfall") || error_message.contains("API") || error_message.contains("rate limit") {
                ErrorResponse::new(crate::errors::codes::ErrorCode::ScryfallApiError, format!("Scryfall API error: {}", e)).into_response()
            } else {
                ErrorResponse::internal_error(format!("Failed to search by name: {}", e)).into_response()
            }
        }
    }
}

/// Get cache statistics
#[utoipa::path(
    get,
    path = "/stats",
    tag = "statistics",
    responses(
        (status = 200, description = "Cache statistics", body = StatsResponse),
        (status = 500, description = "Internal server error", body = StatsResponse)
    )
)]
pub async fn get_stats(State(state): State<AppState>) -> impl IntoResponse {
    info!("Stats request");

    match state.cache_manager.get_stats().await {
        Ok(stats) => {
            info!("Stats: {:?}", stats);
            (StatusCode::OK, Json(ApiResponse::success(stats))).into_response()
        }
        Err(e) => {
            error!("Get stats failed: {}", e);
            ErrorResponse::database_error(format!("Failed to retrieve stats: {}", e)).into_response()
        }
    }
}

/// Force reload bulk data
#[utoipa::path(
    post,
    path = "/admin/reload",
    tag = "admin",
    responses(
        (status = 200, description = "Bulk data reload completed", body = ReloadResponse),
        (status = 500, description = "Reload failed", body = ReloadResponse)
    )
)]
pub async fn admin_reload(State(state): State<AppState>) -> impl IntoResponse {
    info!("Admin reload request");

    match state.bulk_loader.force_load().await {
        Ok(_) => {
            info!("Bulk data reload completed");
            (
                StatusCode::OK,
                Json(ApiResponse::success("Bulk data reload completed".to_string())),
            )
                .into_response()
        }
        Err(e) => {
            error!("Bulk data reload failed: {}", e);
            
            // Map error type to appropriate error code
            let error_message = e.to_string();
            if error_message.contains("scryfall") || error_message.contains("download") || error_message.contains("HTTP") {
                ErrorResponse::new(crate::errors::codes::ErrorCode::ScryfallApiError, format!("Failed to download bulk data: {}", e)).into_response()
            } else if error_message.contains("database") || error_message.contains("connection") {
                ErrorResponse::database_error(format!("Failed to load bulk data into database: {}", e)).into_response()
            } else {
                ErrorResponse::internal_error(format!("Bulk data reload failed: {}", e)).into_response()
            }
        }
    }
}

/// Autocomplete card names
#[utoipa::path(
    get,
    path = "/cards/autocomplete",
    tag = "cards",
    params(AutocompleteParams),
    responses(
        (status = 200, description = "Autocomplete suggestions", body = AutocompleteResponse),
        (status = 400, description = "Bad request - query parameter required", body = AutocompleteResponse),
        (status = 500, description = "Internal server error", body = AutocompleteResponse)
    )
)]
pub async fn autocomplete_cards(
    State(state): State<AppState>,
    Query(params): Query<AutocompleteParams>,
) -> impl IntoResponse {
    let prefix = params.q.trim();
    
    // Return empty results for very short queries
    if prefix.len() < 2 {
        return (
            StatusCode::OK,
            Json(AutocompleteResponse {
                object: "catalog".to_string(),
                data: Vec::new(),
            }),
        )
            .into_response();
    }

    info!("Autocomplete request: prefix='{}'", prefix);

    match state.cache_manager.autocomplete(prefix).await {
        Ok(names) => {
            info!("Autocomplete returned {} names for prefix '{}'", names.len(), prefix);
            (
                StatusCode::OK,
                Json(AutocompleteResponse {
                    object: "catalog".to_string(),
                    data: names,
                }),
            )
                .into_response()
        }
        Err(e) => {
            error!("Autocomplete failed: {}", e);
            ErrorResponse::database_error(format!("Autocomplete failed: {}", e)).into_response()
        }
    }
}
