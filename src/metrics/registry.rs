use lazy_static::lazy_static;
use prometheus::{
    register_gauge, register_histogram_vec, register_int_counter_vec,
    register_int_gauge, Gauge, HistogramVec, IntCounterVec, IntGauge,
};

lazy_static! {
    // HTTP Metrics
    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "http_requests_total",
        "Total HTTP requests",
        &["method", "path", "status"]
    )
    .unwrap();

    pub static ref HTTP_REQUEST_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "path"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .unwrap();

    // Cache Metrics
    pub static ref CACHE_HITS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "cache_hits_total",
        "Total cache hits",
        &["tier"]  // tier: query_cache, database, api
    )
    .unwrap();

    pub static ref CACHE_MISSES_TOTAL: IntCounterVec = register_int_counter_vec!(
        "cache_misses_total",
        "Total cache misses",
        &["tier"]
    )
    .unwrap();

    pub static ref CACHE_SIZE_BYTES: IntGauge = register_int_gauge!(
        "cache_size_bytes",
        "Current cache size in bytes"
    )
    .unwrap();

    // Scryfall API Metrics
    pub static ref SCRYFALL_API_CALLS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "scryfall_api_calls_total",
        "Total calls to Scryfall API",
        &["endpoint"]
    )
    .unwrap();

    pub static ref SCRYFALL_API_ERRORS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "scryfall_api_errors_total",
        "Total Scryfall API errors",
        &["status_code"]
    )
    .unwrap();

    pub static ref SCRYFALL_RATE_LIMIT_WAITS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "scryfall_rate_limit_waits_total",
        "Total times rate limit caused a wait",
        &[]
    )
    .unwrap();

    // Database Metrics
    pub static ref DATABASE_QUERIES_TOTAL: IntCounterVec = register_int_counter_vec!(
        "database_queries_total",
        "Total database queries",
        &["query_type"]  // query_type: select, insert, update, delete
    )
    .unwrap();

    pub static ref DATABASE_QUERY_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "database_query_duration_seconds",
        "Database query duration in seconds",
        &["query_type"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
    )
    .unwrap();

    pub static ref DATABASE_CONNECTIONS_ACTIVE: IntGauge = register_int_gauge!(
        "database_connections_active",
        "Number of active database connections"
    )
    .unwrap();

    pub static ref DATABASE_CONNECTIONS_IDLE: IntGauge = register_int_gauge!(
        "database_connections_idle",
        "Number of idle database connections"
    )
    .unwrap();

    // Business Metrics
    pub static ref CARDS_TOTAL: IntGauge = register_int_gauge!(
        "cards_total",
        "Total number of cards in database"
    )
    .unwrap();

    pub static ref QUERIES_CACHED_TOTAL: IntGauge = register_int_gauge!(
        "queries_cached_total",
        "Total number of cached queries"
    )
    .unwrap();

    pub static ref BULK_DATA_LOAD_DURATION_SECONDS: Gauge = register_gauge!(
        "bulk_data_load_duration_seconds",
        "Time taken for last bulk data load"
    )
    .unwrap();

    pub static ref BULK_DATA_LAST_LOAD_TIMESTAMP: IntGauge = register_int_gauge!(
        "bulk_data_last_load_timestamp",
        "Unix timestamp of last bulk data load"
    )
    .unwrap();

    pub static ref BULK_DATA_CARDS_IMPORTED: IntGauge = register_int_gauge!(
        "bulk_data_cards_imported",
        "Number of cards imported in last bulk data load"
    )
    .unwrap();
}

/// Initialize all metrics (called on startup)
pub fn init_metrics() {
    // Force lazy_static initialization
    lazy_static::initialize(&HTTP_REQUESTS_TOTAL);
    lazy_static::initialize(&HTTP_REQUEST_DURATION_SECONDS);
    lazy_static::initialize(&CACHE_HITS_TOTAL);
    lazy_static::initialize(&CACHE_MISSES_TOTAL);
    lazy_static::initialize(&CACHE_SIZE_BYTES);
    lazy_static::initialize(&SCRYFALL_API_CALLS_TOTAL);
    lazy_static::initialize(&SCRYFALL_API_ERRORS_TOTAL);
    lazy_static::initialize(&SCRYFALL_RATE_LIMIT_WAITS_TOTAL);
    lazy_static::initialize(&DATABASE_QUERIES_TOTAL);
    lazy_static::initialize(&DATABASE_QUERY_DURATION_SECONDS);
    lazy_static::initialize(&DATABASE_CONNECTIONS_ACTIVE);
    lazy_static::initialize(&DATABASE_CONNECTIONS_IDLE);
    lazy_static::initialize(&CARDS_TOTAL);
    lazy_static::initialize(&QUERIES_CACHED_TOTAL);
    lazy_static::initialize(&BULK_DATA_LOAD_DURATION_SECONDS);
    lazy_static::initialize(&BULK_DATA_LAST_LOAD_TIMESTAMP);
    lazy_static::initialize(&BULK_DATA_CARDS_IMPORTED);
}
