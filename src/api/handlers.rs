use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::cache::manager::{CacheManager, CacheStats};
use crate::models::card::Card;
use crate::scryfall::bulk_loader::BulkLoader;

pub type AppState = Arc<AppStateInner>;

pub struct AppStateInner {
    pub cache_manager: CacheManager,
    pub bulk_loader: BulkLoader,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub limit: Option<i64>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
    pub has_more: bool,
}

#[derive(Debug, Deserialize)]
pub struct NamedParams {
    pub fuzzy: Option<String>,
    pub exact: Option<String>,
}

/// Health check endpoint
pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "scryfall-cache",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Search for cards
pub async fn search_cards(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    info!("Search request: query='{}', limit={:?}, page={:?}, page_size={:?}",
        params.q, params.limit, params.page, params.page_size);

    match state.cache_manager.search(&params.q, params.limit).await {
        Ok(cards) => {
            let total = cards.len();

            // Apply pagination
            let page = params.page.unwrap_or(1).max(1);
            let page_size = params.page_size.unwrap_or(100).min(1000).max(1); // Default 100, max 1000

            let start = (page - 1) * page_size;
            let end = (start + page_size).min(total);

            let paginated_cards: Vec<Card> = if start < total {
                cards[start..end].to_vec()
            } else {
                Vec::new()
            };

            let total_pages = (total + page_size - 1) / page_size;
            let has_more = page < total_pages;

            info!("Search returned {} total cards, page {}/{} ({} cards)",
                total, page, total_pages, paginated_cards.len());

            let response = PaginatedResponse {
                data: paginated_cards,
                total,
                page,
                page_size,
                total_pages,
                has_more,
            };

            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Search failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<PaginatedResponse<Card>>::error(e.to_string())),
            )
        }
    }
}

/// Get a specific card by ID
pub async fn get_card(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    info!("Get card request: id={}", id);

    match state.cache_manager.get_card(id).await {
        Ok(Some(card)) => {
            info!("Found card: {}", card.name);
            (StatusCode::OK, Json(ApiResponse::success(card)))
        }
        Ok(None) => {
            info!("Card not found: {}", id);
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<Card>::error("Card not found".to_string())),
            )
        }
        Err(e) => {
            error!("Get card failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Card>::error(e.to_string())),
            )
        }
    }
}

/// Get a card by name (fuzzy or exact)
pub async fn get_card_by_name(
    State(state): State<AppState>,
    Query(params): Query<NamedParams>,
) -> impl IntoResponse {
    let (name, fuzzy) = if let Some(name) = params.fuzzy {
        (name, true)
    } else if let Some(name) = params.exact {
        (name, false)
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<Card>::error(
                "Must provide either 'fuzzy' or 'exact' parameter".to_string(),
            )),
        );
    };

    info!("Get card by name: name='{}', fuzzy={}", name, fuzzy);

    match state.cache_manager.search_by_name(&name, fuzzy).await {
        Ok(Some(card)) => {
            info!("Found card: {}", card.name);
            (StatusCode::OK, Json(ApiResponse::success(card)))
        }
        Ok(None) => {
            info!("Card not found: {}", name);
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<Card>::error("Card not found".to_string())),
            )
        }
        Err(e) => {
            error!("Get card by name failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Card>::error(e.to_string())),
            )
        }
    }
}

/// Get cache statistics
pub async fn get_stats(State(state): State<AppState>) -> impl IntoResponse {
    info!("Stats request");

    match state.cache_manager.get_stats().await {
        Ok(stats) => {
            info!("Stats: {:?}", stats);
            (StatusCode::OK, Json(ApiResponse::success(stats)))
        }
        Err(e) => {
            error!("Get stats failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<CacheStats>::error(e.to_string())),
            )
        }
    }
}

/// Force reload bulk data
pub async fn admin_reload(State(state): State<AppState>) -> impl IntoResponse {
    info!("Admin reload request");

    match state.bulk_loader.force_load().await {
        Ok(_) => {
            info!("Bulk data reload completed");
            (
                StatusCode::OK,
                Json(ApiResponse::success("Bulk data reload completed".to_string())),
            )
        }
        Err(e) => {
            error!("Bulk data reload failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(e.to_string())),
            )
        }
    }
}
