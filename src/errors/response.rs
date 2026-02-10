use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use super::codes::ErrorCode;

/// Structured error response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// Always false for errors
    pub success: bool,
    /// Error details
    pub error: ErrorDetail,
}

/// Error details
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorDetail {
    /// Error code for programmatic handling
    pub code: ErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Request ID for tracing
    pub request_id: String,
    /// Additional context (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code,
                message: message.into(),
                request_id: Uuid::new_v4().to_string(),
                details: None,
            },
        }
    }

    /// Create error with additional details
    pub fn with_details(code: ErrorCode, message: impl Into<String>, details: Value) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code,
                message: message.into(),
                request_id: Uuid::new_v4().to_string(),
                details: Some(details),
            },
        }
    }

    /// Create error with custom request ID
    pub fn with_request_id(
        code: ErrorCode,
        message: impl Into<String>,
        request_id: impl Into<String>,
    ) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code,
                message: message.into(),
                request_id: request_id.into(),
                details: None,
            },
        }
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.error.code.status_code())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        (status, Json(self)).into_response()
    }
}

/// Helper for creating common errors
impl ErrorResponse {
    pub fn invalid_query(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidQuery, message)
    }

    pub fn card_not_found(card_id: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::CardNotFound,
            format!("Card not found: {}", card_id.into()),
        )
    }

    pub fn database_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::DatabaseError, message)
    }

    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ValidationError, message)
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_error_response_serialization() {
        let err = ErrorResponse::new(ErrorCode::InvalidQuery, "Test error");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("INVALID_QUERY"));
        assert!(json.contains("Test error"));
        assert!(json.contains("request_id"));
    }

    #[test]
    fn test_error_with_details() {
        let details = json!({
            "position": 12,
            "expected": ":",
            "got": "="
        });
        let err =
            ErrorResponse::with_details(ErrorCode::InvalidQuery, "Query syntax error", details);
        assert_eq!(err.error.code, ErrorCode::InvalidQuery);
        assert!(err.error.details.is_some());
    }

    #[test]
    fn test_helper_methods() {
        let err = ErrorResponse::card_not_found("abc123");
        assert_eq!(err.error.code, ErrorCode::CardNotFound);
        assert!(err.error.message.contains("abc123"));
    }

    // ========== HTTP STATUS CODE TESTS ==========

    #[test]
    fn test_error_code_status_codes() {
        // Test all ErrorCode variants map to correct HTTP status codes
        assert_eq!(ErrorCode::InvalidQuery.status_code(), 400);
        assert_eq!(ErrorCode::ValidationError.status_code(), 400);
        assert_eq!(ErrorCode::InvalidApiKey.status_code(), 401);
        assert_eq!(ErrorCode::CardNotFound.status_code(), 404);
        assert_eq!(ErrorCode::RateLimitExceeded.status_code(), 429);
        assert_eq!(ErrorCode::InternalError.status_code(), 500);
        assert_eq!(ErrorCode::ScryfallApiError.status_code(), 502);
        assert_eq!(ErrorCode::DatabaseError.status_code(), 503);
    }

    #[test]
    fn test_into_response_status_invalid_query() {
        let error = ErrorResponse::invalid_query("Bad query");
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_into_response_status_validation_error() {
        let error = ErrorResponse::validation_error("Invalid input");
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_into_response_status_unauthorized() {
        let error = ErrorResponse::new(ErrorCode::InvalidApiKey, "Invalid API key");
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_into_response_status_not_found() {
        let error = ErrorResponse::card_not_found("abc123");
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_into_response_status_rate_limit() {
        let error = ErrorResponse::new(ErrorCode::RateLimitExceeded, "Rate limit exceeded");
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn test_into_response_status_internal_error() {
        let error = ErrorResponse::internal_error("Something went wrong");
        let response = error.into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_into_response_status_bad_gateway() {
        let error = ErrorResponse::new(ErrorCode::ScryfallApiError, "Scryfall API failed");
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn test_into_response_status_service_unavailable() {
        let error = ErrorResponse::database_error("Database connection failed");
        let response = error.into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        );
    }
}
