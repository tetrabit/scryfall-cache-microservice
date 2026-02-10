use serde::{Deserialize, Serialize};

/// Query complexity limits to prevent abuse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryLimits {
    /// Maximum query string length (characters)
    pub max_query_length: usize,

    /// Maximum nesting depth for boolean expressions
    pub max_nesting_depth: usize,

    /// Maximum number of OR clauses
    pub max_or_clauses: usize,

    /// Maximum number of results to return
    pub max_results: i64,

    /// Query execution timeout (seconds)
    pub query_timeout_seconds: u64,
}

impl Default for QueryLimits {
    fn default() -> Self {
        Self {
            max_query_length: 1000,
            max_nesting_depth: 5,
            max_or_clauses: 10,
            max_results: 1000,
            query_timeout_seconds: 30,
        }
    }
}

impl QueryLimits {
    /// Create limits from environment variables
    pub fn from_env() -> Self {
        Self {
            max_query_length: std::env::var("QUERY_MAX_LENGTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            max_nesting_depth: std::env::var("QUERY_MAX_NESTING")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            max_or_clauses: std::env::var("QUERY_MAX_OR_CLAUSES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            max_results: std::env::var("QUERY_MAX_RESULTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            query_timeout_seconds: std::env::var("QUERY_TIMEOUT_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        }
    }
}
