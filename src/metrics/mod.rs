pub mod middleware;
pub mod registry;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use prometheus::{Encoder, TextEncoder};

/// Handler for the /metrics endpoint
/// Returns metrics in Prometheus exposition format
pub async fn metrics_handler() -> Response {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();

    let mut buffer = Vec::new();
    match encoder.encode(&metric_families, &mut buffer) {
        Ok(_) => {
            let body = String::from_utf8(buffer).unwrap_or_else(|_| String::from(""));
            (
                StatusCode::OK,
                [("Content-Type", encoder.format_type())],
                body,
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to encode metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to encode metrics: {}", e),
            )
                .into_response()
        }
    }
}

// Re-export commonly used metrics for convenience
pub use registry::{
    BULK_DATA_CARDS_IMPORTED, BULK_DATA_LAST_LOAD_TIMESTAMP, BULK_DATA_LOAD_DURATION_SECONDS,
    CACHE_HITS_TOTAL, CACHE_MISSES_TOTAL, CACHE_SIZE_BYTES, CARDS_TOTAL,
    DATABASE_CONNECTIONS_ACTIVE, DATABASE_CONNECTIONS_IDLE, DATABASE_QUERIES_TOTAL,
    DATABASE_QUERY_DURATION_SECONDS, HTTP_REQUESTS_TOTAL, HTTP_REQUEST_DURATION_SECONDS,
    QUERIES_CACHED_TOTAL, SCRYFALL_API_CALLS_TOTAL, SCRYFALL_API_ERRORS_TOTAL,
    SCRYFALL_RATE_LIMIT_WAITS_TOTAL,
};
