use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::Service;

// Helper to create test app
async fn create_test_app() -> axum::Router {
    use scryfall_cache::{api, cache, config, db, query, scryfall};
    use std::sync::Arc;

    let config = config::Config::from_env().expect("Failed to load configuration from environment");

    let db_pool = db::init_database(&config.database)
        .await
        .expect("Failed to connect to database");

    let scryfall_client = scryfall::client::ScryfallClient::new(&config.scryfall);
    let cache_manager = Arc::new(cache::manager::CacheManager::new(
        None, // Redis optional in tests
        db_pool.clone(),
        scryfall_client,
        config.cache.query_cache_ttl_hours as i32,
    ));
    let bulk_loader =
        scryfall::bulk_loader::BulkLoader::new(db_pool.clone(), config.scryfall.clone());
    let query_validator =
        scryfall_cache::query::QueryValidator::new(query::QueryLimits::from_env());

    // GraphQL schema is part of AppStateInner and needs access to shared state.
    let graphql_schema = scryfall_cache::graphql::create_schema(
        cache_manager.clone(),
        Arc::new(bulk_loader.clone()),
    );

    let state = Arc::new(api::handlers::AppStateInner {
        cache_manager,
        bulk_loader,
        query_validator,
        graphql_schema,
        instance_id: config.server.instance_id.clone(),
    });

    api::routes::create_router(state)
}

// Helper to send request and parse JSON response
async fn send_json_request(app: &mut axum::Router, method: &str, uri: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap();

    let response = app.call(request).await.unwrap();
    let status = response.status();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap_or(json!({}));

    (status, json)
}

// Helper to send JSON request with JSON body
async fn send_json_body_request(
    app: &mut axum::Router,
    method: &str,
    uri: &str,
    body: Value,
) -> (StatusCode, Value) {
    let bytes = serde_json::to_vec(&body).unwrap();
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(bytes))
        .unwrap();

    let response = app.call(request).await.unwrap();
    let status = response.status();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap_or(json!({}));

    (status, json)
}

#[tokio::test]
async fn test_health_endpoint() {
    let mut app = create_test_app().await;
    let (status, body) = send_json_request(&mut app, "GET", "/health").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "scryfall-cache");
    assert!(body["instance_id"].is_string());
}

#[tokio::test]
async fn test_health_live_endpoint() {
    let mut app = create_test_app().await;
    let (status, body) = send_json_request(&mut app, "GET", "/health/live").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "alive");
    assert_eq!(body["service"], "scryfall-cache");
    assert!(body["instance_id"].is_string());
}

#[tokio::test]
async fn test_health_ready_endpoint() {
    let mut app = create_test_app().await;
    let (status, body) = send_json_request(&mut app, "GET", "/health/ready").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ready");
    assert!(body["checks"].is_object());
    assert_eq!(body["checks"]["database"], "ok");
}

#[tokio::test]
async fn test_admin_overview_endpoint() {
    let mut app = create_test_app().await;
    let (status, body) = send_json_request(&mut app, "GET", "/api/admin/stats/overview").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert!(body["data"].is_object());
    assert_eq!(body["data"]["service"], "scryfall-cache");
    assert!(body["data"]["cards_total"].is_number());
}

#[tokio::test]
async fn test_search_cards_basic() {
    let mut app = create_test_app().await;
    let (status, body) = send_json_request(&mut app, "GET", "/cards/search?q=c:r").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert!(body["data"].is_object());
    assert!(body["data"]["data"].is_array());
}

#[tokio::test]
async fn test_search_cards_invalid_query() {
    let mut app = create_test_app().await;
    let (status, body) = send_json_request(&mut app, "GET", "/cards/search?q=((((").await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["success"], false);
    assert!(body["error"].is_object());
    assert!(body["error"]["message"].is_string());
}

#[tokio::test]
async fn test_search_cards_pagination() {
    let mut app = create_test_app().await;
    let (status, body) =
        send_json_request(&mut app, "GET", "/cards/search?q=c:u&page=1&page_size=10").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["page"], 1);
    assert_eq!(body["data"]["page_size"], 10);
    assert!(body["data"]["total"].is_number());
}

#[tokio::test]
async fn test_batch_get_cards() {
    let mut app = create_test_app().await;

    // Find one card ID via search.
    let (status, search_body) = send_json_request(&mut app, "GET", "/cards/search?q=sol+ring").await;
    assert_eq!(status, StatusCode::OK);

    let first_id = search_body["data"]["data"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["id"].as_str())
        .expect("expected at least one search result with an id");

    let (status, body) = send_json_body_request(
        &mut app,
        "POST",
        "/cards/batch",
        json!({
            "ids": [first_id],
            "fetch_missing": false
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert!(body["data"]["cards"].is_array());
    assert_eq!(body["data"]["cards"][0]["id"], first_id);
}

#[tokio::test]
async fn test_batch_get_cards_by_name() {
    let mut app = create_test_app().await;

    // Use a name from a known search result so the test doesn't depend on hard-coded data.
    let (status, search_body) = send_json_request(&mut app, "GET", "/cards/search?q=sol+ring").await;
    assert_eq!(status, StatusCode::OK);

    let first_name = search_body["data"]["data"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["name"].as_str())
        .expect("expected at least one search result with a name");

    let (status, body) = send_json_body_request(
        &mut app,
        "POST",
        "/cards/named/batch",
        json!({
            "names": [first_name],
            "fuzzy": false
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["results"][0]["name"], first_name);
    assert_eq!(body["data"]["results"][0]["card"]["name"], first_name);
}

#[tokio::test]
async fn test_batch_execute_queries() {
    let mut app = create_test_app().await;

    let (status, body) = send_json_body_request(
        &mut app,
        "POST",
        "/queries/batch",
        json!({
            "queries": [
                { "id": "q1", "query": "c:r", "page": 1, "page_size": 5 },
                { "id": "q2", "query": "c:u", "page": 1, "page_size": 5 }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert!(body["data"]["results"].is_array());
    assert_eq!(body["data"]["results"][0]["id"], "q1");
    assert!(body["data"]["results"][0]["success"].is_boolean());
}

#[tokio::test]
async fn test_get_card_by_id() {
    let mut app = create_test_app().await;

    // First search for a card to get valid ID
    let (_, search_body) = send_json_request(&mut app, "GET", "/cards/search?q=sol+ring").await;

    if let Some(cards) = search_body["data"]["data"].as_array() {
        if let Some(first_card) = cards.first() {
            if let Some(card_id) = first_card["id"].as_str() {
                let uri = format!("/cards/{}", card_id);
                let (status, body) = send_json_request(&mut app, "GET", &uri).await;

                assert_eq!(status, StatusCode::OK);
                assert_eq!(body["success"], true);
                assert!(body["data"]["id"].is_string());
            }
        }
    }
}

#[tokio::test]
async fn test_get_card_not_found() {
    let mut app = create_test_app().await;
    let fake_uuid = "00000000-0000-0000-0000-000000000000";
    let uri = format!("/cards/{}", fake_uuid);
    let (status, body) = send_json_request(&mut app, "GET", &uri).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["success"], false);
    assert_eq!(body["error"]["code"], "CARD_NOT_FOUND");
}

#[tokio::test]
async fn test_named_card_exact() {
    let mut app = create_test_app().await;
    let (status, body) =
        send_json_request(&mut app, "GET", "/cards/named?exact=Lightning%20Bolt").await;

    if status == StatusCode::OK {
        assert_eq!(body["success"], true);
        assert!(body["data"]["name"].as_str().unwrap().contains("Lightning"));
    }
}

#[tokio::test]
async fn test_named_card_fuzzy() {
    let mut app = create_test_app().await;
    let (status, body) =
        send_json_request(&mut app, "GET", "/cards/named?fuzzy=light%20bolt").await;

    if status == StatusCode::OK {
        assert_eq!(body["success"], true);
        assert!(body["data"]["name"].is_string());
    }
}

#[tokio::test]
async fn test_autocomplete() {
    let mut app = create_test_app().await;
    let (status, body) =
        send_json_request(&mut app, "GET", "/cards/autocomplete?q=lightning").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["object"], "catalog");
    assert!(body["data"].is_array());
}

#[tokio::test]
async fn test_cache_stats() {
    let mut app = create_test_app().await;
    let (status, body) = send_json_request(&mut app, "GET", "/stats").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert!(body["data"]["total_cards"].is_number());
    assert!(body["data"]["total_cache_entries"].is_number());
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let mut app = create_test_app().await;

    let request = Request::builder()
        .method("GET")
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();

    let response = app.call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();

    // Check for Prometheus format metrics
    assert!(text.contains("# HELP"));
    assert!(text.contains("# TYPE"));
}

#[tokio::test]
async fn test_query_validation_max_length() {
    let mut app = create_test_app().await;
    let long_query = "a".repeat(2000); // Exceeds default 1000 char limit
    let uri = format!("/cards/search?q={}", long_query);
    let (status, body) = send_json_request(&mut app, "GET", &uri).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["success"], false);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("too long"));
}

#[tokio::test]
async fn test_error_response_structure() {
    let mut app = create_test_app().await;
    let (_, body) = send_json_request(&mut app, "GET", "/cards/search?q=((((").await;

    // Verify ErrorResponse structure
    assert_eq!(body["success"], false);
    assert!(body["error"].is_object());
    assert!(body["error"]["code"].is_string());
    assert!(body["error"]["message"].is_string());
    assert!(body["error"]["request_id"].is_string());
}
