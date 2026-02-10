use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use super::handlers::{
    admin_reload, autocomplete_cards, get_card, get_card_by_name, get_stats, health, health_live,
    health_ready, search_cards, AppState,
};
use super::middleware::logging_middleware;
use super::openapi::ApiDoc;
use crate::metrics;

pub fn create_router(state: AppState) -> Router {
    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health check
        .route("/health", get(health))
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        // Card search endpoints
        .route("/cards/search", get(search_cards))
        .route("/cards/named", get(get_card_by_name))
        .route("/cards/autocomplete", get(autocomplete_cards))
        .route("/cards/:id", get(get_card))
        // Stats endpoint
        .route("/stats", get(get_stats))
        // Metrics endpoint (Prometheus)
        .route("/metrics", get(metrics::metrics_handler))
        // Admin endpoints
        .route("/admin/reload", post(admin_reload))
        // OpenAPI documentation
        .merge(SwaggerUi::new("/api-docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Add middleware (order matters: compression -> logging -> metrics -> cors -> trace)
        .layer(CompressionLayer::new())
        .layer(middleware::from_fn(logging_middleware))
        .layer(middleware::from_fn(metrics::middleware::track_metrics))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        // Add shared state
        .with_state(state)
}
