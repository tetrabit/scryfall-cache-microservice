use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

/// Middleware to log all HTTP requests and responses with structured data
pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let request_id = Uuid::new_v4().to_string();
    
    // Extract request information
    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let path = request.uri().path().to_string();
    let query = request.uri().query().unwrap_or("").to_string();
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Log incoming request
    info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        query = %sanitize_query(&query),
        user_agent = %user_agent,
        "Incoming request"
    );

    // Process request
    let response = next.run(request).await;
    
    // Calculate duration
    let duration = start.elapsed();
    let status = response.status();
    
    // Log response
    if status.is_success() {
        info!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Request completed"
        );
    } else if status.is_client_error() {
        warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Request failed (client error)"
        );
    } else if status.is_server_error() {
        warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Request failed (server error)"
        );
    }

    response
}

/// Sanitize query parameters to hide sensitive data
fn sanitize_query(query: &str) -> String {
    if query.is_empty() {
        return String::new();
    }
    
    // Replace potential API keys or tokens with regex-like approach
    let mut result = query.to_string();
    for (key, replacement) in [
        ("api_key", "api_key=***"),
        ("token", "token=***"),
        ("password", "password=***"),
        ("secret", "secret=***"),
    ] {
        let pattern = format!("{}=", key);
        if let Some(start) = result.find(&pattern) {
            let value_start = start + pattern.len();
            // Find end of value (next & or end of string)
            let value_end = result[value_start..]
                .find('&')
                .map(|i| value_start + i)
                .unwrap_or(result.len());
            result.replace_range(start..value_end, replacement);
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_query() {
        assert_eq!(sanitize_query(""), "");
        assert_eq!(sanitize_query("q=sol+ring"), "q=sol+ring");
        assert_eq!(sanitize_query("api_key=secret123"), "api_key=***");
        assert_eq!(
            sanitize_query("q=test&api_key=secret&limit=10"),
            "q=test&api_key=***&limit=10"
        );
    }
}
