use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use super::handlers::{
    admin_reload, get_card, get_card_by_name, get_stats, health, search_cards, AppState,
};

pub fn create_router(state: AppState) -> Router {
    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health check
        .route("/health", get(health))
        // Card search endpoints
        .route("/cards/search", get(search_cards))
        .route("/cards/named", get(get_card_by_name))
        .route("/cards/:id", get(get_card))
        // Stats endpoint
        .route("/stats", get(get_stats))
        // Admin endpoints
        .route("/admin/reload", post(admin_reload))
        // Add middleware
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        // Add shared state
        .with_state(state)
}
