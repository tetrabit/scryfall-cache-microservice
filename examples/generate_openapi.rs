use scryfall_cache::api::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let doc = ApiDoc::openapi();

    // Serialize to JSON
    let json =
        serde_json::to_string_pretty(&doc).expect("Failed to serialize OpenAPI spec to JSON");

    println!("{}", json);

    eprintln!("✅ OpenAPI specification generated successfully");
}
