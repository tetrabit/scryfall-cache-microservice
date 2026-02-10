use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::cache::manager::{CacheManager, CacheStats};
use crate::errors::{ErrorCode, ErrorResponse};
use crate::models::card::Card;
use crate::query::{QueryParser, QueryValidator};
use crate::scryfall::bulk_loader::BulkLoader;

lazy_static::lazy_static! {
    static ref START_TIME: Instant = Instant::now();
}

pub type AppState = Arc<AppStateInner>;

pub struct AppStateInner {
    pub cache_manager: CacheManager,
    pub bulk_loader: BulkLoader,
    pub query_validator: QueryValidator,
    pub instance_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminOverview {
    pub service: String,
    pub version: String,
    pub instance_id: String,
    pub cards_total: i64,
    pub cache_entries_total: i64,
    pub bulk_last_import: Option<String>,
    pub bulk_reload_recommended: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminOverviewResponse {
    pub success: bool,
    pub data: Option<AdminOverview>,
    pub error: Option<crate::errors::response::ErrorDetail>,
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

/// Batch card lookup request
#[derive(Debug, Deserialize, ToSchema)]
pub struct BatchCardsRequest {
    /// List of card UUIDs to fetch
    pub ids: Vec<Uuid>,
    /// If true, missing cards will be fetched from Scryfall (via /cards/collection) and cached
    pub fetch_missing: Option<bool>,
}

/// Batch card lookup response payload
#[derive(Debug, Serialize, ToSchema)]
pub struct BatchCardsData {
    /// Cards found (in the same order as requested IDs; missing cards omitted)
    pub cards: Vec<Card>,
    /// IDs that could not be found (unique)
    pub missing_ids: Vec<Uuid>,
}

/// Batch card list response
#[derive(Debug, Serialize, ToSchema)]
pub struct BatchCardsResponse {
    pub success: bool,
    pub data: Option<BatchCardsData>,
    pub error: Option<crate::errors::response::ErrorDetail>,
}

/// Batch named lookup request
#[derive(Debug, Deserialize, ToSchema)]
pub struct BatchNamedRequest {
    /// List of card names
    pub names: Vec<String>,
    /// If true, use fuzzy matching (slower, may call upstream more often)
    pub fuzzy: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BatchNamedResult {
    pub name: String,
    pub card: Option<Card>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BatchNamedData {
    pub results: Vec<BatchNamedResult>,
    pub not_found: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BatchNamedResponse {
    pub success: bool,
    pub data: Option<BatchNamedData>,
    pub error: Option<crate::errors::response::ErrorDetail>,
}

/// Batch query execution request
#[derive(Debug, Deserialize, ToSchema)]
pub struct BatchQueryItem {
    pub id: String,
    pub query: String,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BatchQueriesRequest {
    pub queries: Vec<BatchQueryItem>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BatchQueryResult {
    pub id: String,
    pub success: bool,
    pub data: Option<PaginatedResponse<Card>>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BatchQueriesData {
    pub results: Vec<BatchQueryResult>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BatchQueriesResponse {
    pub success: bool,
    pub data: Option<BatchQueriesData>,
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
pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    // Backwards-compatible "info" endpoint. This is intentionally liveness-style: it does not
    // perform dependency checks. Use /health/ready for readiness.
    Json(serde_json::json!({
        "status": "healthy",
        "service": "scryfall-cache",
        "version": env!("CARGO_PKG_VERSION"),
        "instance_id": state.instance_id.clone(),
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

/// Liveness endpoint (no dependency checks)
#[utoipa::path(
    get,
    path = "/health/live",
    tag = "health",
    responses(
        (status = 200, description = "Service is alive", body = serde_json::Value)
    )
)]
pub async fn health_live(State(state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "alive",
        "service": "scryfall-cache",
        "version": env!("CARGO_PKG_VERSION"),
        "instance_id": state.instance_id.clone(),
        "uptime_seconds": START_TIME.elapsed().as_secs(),
    }))
}

/// Readiness endpoint (dependency checks)
#[utoipa::path(
    get,
    path = "/health/ready",
    tag = "health",
    responses(
        (status = 200, description = "Service is ready to receive traffic", body = serde_json::Value),
        (status = 503, description = "Service is not ready", body = serde_json::Value)
    )
)]
pub async fn health_ready(State(state): State<AppState>) -> impl IntoResponse {
    let mut checks = serde_json::Map::new();

    let db_ok = match state.cache_manager.test_database_connection().await {
        Ok(()) => {
            checks.insert(
                "database".to_string(),
                serde_json::Value::String("ok".to_string()),
            );
            true
        }
        Err(e) => {
            checks.insert(
                "database".to_string(),
                serde_json::Value::String(format!("error: {}", e)),
            );
            false
        }
    };

    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(serde_json::json!({
            "status": if db_ok { "ready" } else { "not_ready" },
            "service": "scryfall-cache",
            "version": env!("CARGO_PKG_VERSION"),
            "instance_id": state.instance_id.clone(),
            "uptime_seconds": START_TIME.elapsed().as_secs(),
            "checks": checks,
        })),
    )
}

/// Admin: overview stats for dashboard
#[utoipa::path(
    get,
    path = "/api/admin/stats/overview",
    tag = "admin",
    responses(
        (status = 200, description = "Admin overview stats", body = AdminOverviewResponse),
        (status = 500, description = "Internal server error", body = AdminOverviewResponse)
    )
)]
pub async fn admin_stats_overview(State(state): State<AppState>) -> impl IntoResponse {
    let stats = match state.cache_manager.get_stats().await {
        Ok(s) => s,
        Err(e) => {
            return ErrorResponse::database_error(format!("Failed to load stats: {}", e))
                .into_response();
        }
    };

    let bulk_last_import = match state.bulk_loader.last_import_timestamp().await {
        Ok(ts) => ts.map(|t| t.to_string()),
        Err(e) => {
            return ErrorResponse::database_error(format!(
                "Failed to load bulk import timestamp: {}",
                e
            ))
            .into_response();
        }
    };

    let bulk_reload_recommended = match state.bulk_loader.should_load().await {
        Ok(v) => v,
        Err(e) => {
            return ErrorResponse::database_error(format!(
                "Failed to determine bulk load status: {}",
                e
            ))
            .into_response();
        }
    };

    let overview = AdminOverview {
        service: "scryfall-cache".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        instance_id: state.instance_id.clone(),
        cards_total: stats.total_cards,
        cache_entries_total: stats.total_cache_entries,
        bulk_last_import,
        bulk_reload_recommended,
    };

    (StatusCode::OK, Json(ApiResponse::success(overview))).into_response()
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
    info!(
        "Search request: query='{}', limit={:?}, page={:?}, page_size={:?}",
        params.q, params.limit, params.page, params.page_size
    );

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
            return ErrorResponse::invalid_query(format!("Query parse error: {}", e))
                .into_response();
        }
    }

    // Use pagination parameters
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(100).min(1000).max(1);

    // Use the new paginated search which is much faster
    match state
        .cache_manager
        .search_paginated(&params.q, page, page_size)
        .await
    {
        Ok((cards, total)) => {
            let total_pages = total.div_ceil(page_size);
            let has_more = page < total_pages;

            info!(
                "Search returned {} cards (page {}/{}), {} total matches",
                cards.len(),
                page,
                total_pages,
                total
            );

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
            if error_message.contains("Scryfall API error")
                || error_message.contains("Scryfall API unavailable")
                || error_message.contains("Circuit breaker")
            {
                ErrorResponse::new(
                    ErrorCode::ScryfallApiError,
                    format!("Upstream Scryfall failure: {}", e),
                )
                .into_response()
            } else if error_message.contains("database")
                || error_message.contains("connection")
                || error_message.contains("pool")
            {
                ErrorResponse::database_error(format!("Database error during search: {}", e))
                    .into_response()
            } else {
                ErrorResponse::internal_error(format!("Search failed: {}", e)).into_response()
            }
        }
    }
}

/// Batch fetch cards by ID
#[utoipa::path(
    post,
    path = "/cards/batch",
    tag = "cards",
    request_body = BatchCardsRequest,
    responses(
        (status = 200, description = "Batch card lookup result", body = BatchCardsResponse),
        (status = 400, description = "Bad request", body = BatchCardsResponse),
        (status = 500, description = "Internal server error", body = BatchCardsResponse)
    )
)]
pub async fn batch_get_cards(
    State(state): State<AppState>,
    Json(req): Json<BatchCardsRequest>,
) -> impl IntoResponse {
    let max_ids: usize = std::env::var("BATCH_MAX_IDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);

    if req.ids.is_empty() {
        return ErrorResponse::validation_error("ids must not be empty").into_response();
    }
    if req.ids.len() > max_ids {
        return ErrorResponse::validation_error(format!(
            "too many ids: {} (max {})",
            req.ids.len(),
            max_ids
        ))
        .into_response();
    }

    let fetch_missing = req.fetch_missing.unwrap_or(false);

    match state
        .cache_manager
        .get_cards_batch(&req.ids, fetch_missing)
        .await
    {
        Ok((cards, missing_ids)) => {
            let data = BatchCardsData { cards, missing_ids };
            (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
        }
        Err(e) => {
            error!("Batch get cards failed: {}", e);
            ErrorResponse::internal_error(format!("Batch get cards failed: {}", e)).into_response()
        }
    }
}

/// Batch get cards by name
#[utoipa::path(
    post,
    path = "/cards/named/batch",
    tag = "cards",
    request_body = BatchNamedRequest,
    responses(
        (status = 200, description = "Batch named card lookup result", body = BatchNamedResponse),
        (status = 400, description = "Bad request", body = BatchNamedResponse),
        (status = 500, description = "Internal server error", body = BatchNamedResponse)
    )
)]
pub async fn batch_get_cards_by_name(
    State(state): State<AppState>,
    Json(req): Json<BatchNamedRequest>,
) -> impl IntoResponse {
    let max_names: usize = std::env::var("BATCH_MAX_NAMES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);

    if req.names.is_empty() {
        return ErrorResponse::validation_error("names must not be empty").into_response();
    }
    if req.names.len() > max_names {
        return ErrorResponse::validation_error(format!(
            "too many names: {} (max {})",
            req.names.len(),
            max_names
        ))
        .into_response();
    }

    let fuzzy = req.fuzzy.unwrap_or(true);

    let parallelism: usize = std::env::var("BATCH_PARALLELISM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4)
        .max(1)
        .min(32);

    let mut indexed: Vec<(usize, BatchNamedResult)> = futures::stream::iter(
        req.names
            .into_iter()
            .enumerate()
            .map(|(idx, name)| (idx, name)),
    )
    .map(|(idx, name)| {
        let state = state.clone();
        async move {
            let res = match state.cache_manager.search_by_name(&name, fuzzy).await {
                Ok(card_opt) => BatchNamedResult {
                    name,
                    card: card_opt,
                },
                Err(e) => {
                    error!("Batch named lookup failed: {}", e);
                    BatchNamedResult { name, card: None }
                }
            };
            (idx, res)
        }
    })
    .buffer_unordered(parallelism)
    .collect()
    .await;

    indexed.sort_by_key(|(idx, _)| *idx);
    let mut results = Vec::with_capacity(indexed.len());
    let mut not_found = Vec::new();
    for (_idx, item) in indexed {
        if item.card.is_none() {
            not_found.push(item.name.clone());
        }
        results.push(item);
    }

    let data = BatchNamedData { results, not_found };
    (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
}

/// Batch execute queries
#[utoipa::path(
    post,
    path = "/queries/batch",
    tag = "cards",
    request_body = BatchQueriesRequest,
    responses(
        (status = 200, description = "Batch query execution result", body = BatchQueriesResponse),
        (status = 400, description = "Bad request", body = BatchQueriesResponse),
        (status = 500, description = "Internal server error", body = BatchQueriesResponse)
    )
)]
pub async fn batch_execute_queries(
    State(state): State<AppState>,
    Json(req): Json<BatchQueriesRequest>,
) -> impl IntoResponse {
    let max_queries: usize = std::env::var("BATCH_MAX_QUERIES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);

    if req.queries.is_empty() {
        return ErrorResponse::validation_error("queries must not be empty").into_response();
    }
    if req.queries.len() > max_queries {
        return ErrorResponse::validation_error(format!(
            "too many queries: {} (max {})",
            req.queries.len(),
            max_queries
        ))
        .into_response();
    }

    let parallelism: usize = std::env::var("BATCH_PARALLELISM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4)
        .max(1)
        .min(32);

    let mut indexed: Vec<(usize, BatchQueryResult)> =
        futures::stream::iter(req.queries.into_iter().enumerate())
            .map(|(idx, item)| {
                let state = state.clone();
                async move {
                    let id = item.id;
                    let query = item.query;

                    // Validate query string
                    if let Err(e) = state.query_validator.validate_query_string(&query) {
                        return (
                            idx,
                            BatchQueryResult {
                                id,
                                success: false,
                                data: None,
                                error: Some(e.to_string()),
                            },
                        );
                    }

                    // Parse and validate query AST
                    match QueryParser::parse(&query) {
                        Ok(ast) => {
                            if let Err(e) = state.query_validator.validate_ast(&ast) {
                                return (
                                    idx,
                                    BatchQueryResult {
                                        id,
                                        success: false,
                                        data: None,
                                        error: Some(e.to_string()),
                                    },
                                );
                            }
                        }
                        Err(e) => {
                            return (
                                idx,
                                BatchQueryResult {
                                    id,
                                    success: false,
                                    data: None,
                                    error: Some(format!("Query parse error: {}", e)),
                                },
                            );
                        }
                    }

                    let page = item.page.unwrap_or(1).max(1);
                    let page_size = item.page_size.unwrap_or(100).min(1000).max(1);

                    match state.cache_manager.search_paginated(&query, page, page_size).await {
                        Ok((cards, total)) => {
                            let total_pages = total.div_ceil(page_size);
                            let has_more = page < total_pages;
                            let data = PaginatedResponse {
                                data: cards,
                                total,
                                page,
                                page_size,
                                total_pages,
                                has_more,
                            };
                            (
                                idx,
                                BatchQueryResult {
                                    id,
                                    success: true,
                                    data: Some(data),
                                    error: None,
                                },
                            )
                        }
                        Err(e) => (
                            idx,
                            BatchQueryResult {
                                id,
                                success: false,
                                data: None,
                                error: Some(e.to_string()),
                            },
                        ),
                    }
                }
            })
            .buffer_unordered(parallelism)
            .collect()
            .await;

    indexed.sort_by_key(|(idx, _)| *idx);
    let results: Vec<BatchQueryResult> = indexed.into_iter().map(|(_i, r)| r).collect();

    let data = BatchQueriesData { results };
    (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
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
pub async fn get_card(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
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
            let error_message = e.to_string();
            if error_message.contains("Scryfall API error")
                || error_message.contains("Scryfall API unavailable")
                || error_message.contains("Circuit breaker")
            {
                ErrorResponse::new(
                    ErrorCode::ScryfallApiError,
                    format!("Upstream Scryfall failure: {}", e),
                )
                .into_response()
            } else if error_message.contains("database")
                || error_message.contains("connection")
                || error_message.contains("pool")
            {
                ErrorResponse::database_error(format!("Failed to fetch card: {}", e))
                    .into_response()
            } else {
                ErrorResponse::internal_error(format!("Failed to fetch card: {}", e))
                    .into_response()
            }
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
        return ErrorResponse::validation_error("Must provide either 'fuzzy' or 'exact' parameter")
            .into_response();
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
            if error_message.contains("database")
                || error_message.contains("connection")
                || error_message.contains("pool")
            {
                ErrorResponse::database_error(format!("Database error: {}", e)).into_response()
            } else if error_message.contains("scryfall")
                || error_message.contains("API")
                || error_message.contains("rate limit")
            {
                ErrorResponse::new(
                    ErrorCode::ScryfallApiError,
                    format!("Scryfall API error: {}", e),
                )
                .into_response()
            } else {
                ErrorResponse::internal_error(format!("Failed to search by name: {}", e))
                    .into_response()
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
            ErrorResponse::database_error(format!("Failed to retrieve stats: {}", e))
                .into_response()
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
                Json(ApiResponse::success(
                    "Bulk data reload completed".to_string(),
                )),
            )
                .into_response()
        }
        Err(e) => {
            error!("Bulk data reload failed: {}", e);

            // Map error type to appropriate error code
            let error_message = e.to_string();
            if error_message.contains("scryfall")
                || error_message.contains("download")
                || error_message.contains("HTTP")
            {
                ErrorResponse::new(
                    crate::errors::codes::ErrorCode::ScryfallApiError,
                    format!("Failed to download bulk data: {}", e),
                )
                .into_response()
            } else if error_message.contains("database") || error_message.contains("connection") {
                ErrorResponse::database_error(format!(
                    "Failed to load bulk data into database: {}",
                    e
                ))
                .into_response()
            } else {
                ErrorResponse::internal_error(format!("Bulk data reload failed: {}", e))
                    .into_response()
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
            info!(
                "Autocomplete returned {} names for prefix '{}'",
                names.len(),
                prefix
            );
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
