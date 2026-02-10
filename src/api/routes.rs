use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use super::handlers::{
    admin_reload, admin_stats_overview, autocomplete_cards, batch_execute_queries, batch_get_cards,
    batch_get_cards_by_name, get_card, get_card_by_name, get_stats, graphql_playground, health,
    health_live, health_ready, search_cards, AppState,
};
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use async_graphql::{Request as GraphQLRequest, Response as GraphQLResponse};
use super::middleware::logging_middleware;
use super::openapi::ApiDoc;
use crate::metrics;

pub fn create_router(state: AppState) -> Router {
    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Clone GraphQL schema for extension layer
    let graphql_schema = state.graphql_schema.clone();

    Router::new()
        // Health check
        .route("/health", get(health))
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        // GraphQL endpoint (using async-graphql-axum integration)
        .route(
            "/graphql",
            get(graphql_query_handler).post(graphql_query_handler),
        )
        .route("/graphql/playground", get(graphql_playground))
        // Add GraphQL schema as extension for the /graphql route
        .layer(axum::Extension(graphql_schema))
        // Admin API endpoints (for web UI)
        .route("/api/admin/stats/overview", get(admin_stats_overview))
        // Card search endpoints
        .route("/cards/search", get(search_cards))
        .route("/cards/named", get(get_card_by_name))
        .route("/cards/named/batch", post(batch_get_cards_by_name))
        .route("/cards/autocomplete", get(autocomplete_cards))
        .route("/cards/:id", get(get_card))
        .route("/cards/batch", post(batch_get_cards))
        .route("/queries/batch", post(batch_execute_queries))
        // Stats endpoint
        .route("/stats", get(get_stats))
        // Metrics endpoint (Prometheus)
        .route("/metrics", get(metrics::metrics_handler))
        // Admin endpoints
        .route("/admin/reload", post(admin_reload))
        // Admin panel (static files). Build the frontend into admin-panel/dist.
        // Note: /admin/reload remains an API endpoint and should take precedence.
        .nest_service(
            "/admin",
            ServeDir::new("admin-panel/dist")
                .not_found_service(ServeFile::new("admin-panel/dist/index.html")),
        )
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

/// GraphQL query handler
async fn graphql_query_handler(
    axum::Extension(schema): axum::Extension<crate::graphql::GraphQLSchema>,
    Json(req): Json<GraphQLRequest>,
) -> Json<GraphQLResponse> {
    let response = schema.execute(req).await;
    Json(response)
}
