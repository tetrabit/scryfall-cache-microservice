use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

/// Error codes for structured API responses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    /// Query syntax error
    #[serde(rename = "INVALID_QUERY")]
    InvalidQuery,
    
    /// Card ID not found
    #[serde(rename = "CARD_NOT_FOUND")]
    CardNotFound,
    
    /// Too many requests / rate limit exceeded
    #[serde(rename = "RATE_LIMIT_EXCEEDED")]
    RateLimitExceeded,
    
    /// Database connection or query error
    #[serde(rename = "DATABASE_ERROR")]
    DatabaseError,
    
    /// Upstream Scryfall API failure
    #[serde(rename = "SCRYFALL_API_ERROR")]
    ScryfallApiError,
    
    /// Authentication failed
    #[serde(rename = "INVALID_API_KEY")]
    InvalidApiKey,
    
    /// Input validation failed
    #[serde(rename = "VALIDATION_ERROR")]
    ValidationError,
    
    /// Internal server error
    #[serde(rename = "INTERNAL_ERROR")]
    InternalError,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuery => write!(f, "INVALID_QUERY"),
            Self::CardNotFound => write!(f, "CARD_NOT_FOUND"),
            Self::RateLimitExceeded => write!(f, "RATE_LIMIT_EXCEEDED"),
            Self::DatabaseError => write!(f, "DATABASE_ERROR"),
            Self::ScryfallApiError => write!(f, "SCRYFALL_API_ERROR"),
            Self::InvalidApiKey => write!(f, "INVALID_API_KEY"),
            Self::ValidationError => write!(f, "VALIDATION_ERROR"),
            Self::InternalError => write!(f, "INTERNAL_ERROR"),
        }
    }
}

impl ErrorCode {
    /// Get HTTP status code for this error
    pub fn status_code(&self) -> u16 {
        match self {
            Self::InvalidQuery => 400,
            Self::CardNotFound => 404,
            Self::RateLimitExceeded => 429,
            Self::DatabaseError => 503,
            Self::ScryfallApiError => 502,
            Self::InvalidApiKey => 401,
            Self::ValidationError => 400,
            Self::InternalError => 500,
        }
    }
}
