#!/bin/bash
# Generate static OpenAPI specification file

set -e

echo "=== Generating OpenAPI Specification ==="
echo

# Build the project first
echo "Building project..."
cargo build --release

# Create a temporary Rust program to output the OpenAPI spec
cat > /tmp/generate_openapi.rs << 'EOF'
use scryfall_cache::api::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let doc = ApiDoc::openapi();
    let yaml = serde_yaml::to_string(&doc).expect("Failed to serialize OpenAPI spec");
    println!("{}", yaml);
}
EOF

# Check if we can use cargo-run or need a different approach
if cargo run --example generate_openapi 2>/dev/null; then
    echo "✅ OpenAPI spec generated via example"
else
    echo "Note: To generate static openapi.yaml, add this to your project:"
    echo
    echo "Create: examples/generate_openapi.rs"
    echo "---"
    cat << 'EXAMPLE'
use scryfall_cache::api::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let doc = ApiDoc::openapi();
    let yaml = serde_yaml::to_string(&doc).expect("Failed to serialize OpenAPI spec");
    println!("{}", yaml);
}
EXAMPLE
    echo "---"
    echo
    echo "Then run: cargo run --example generate_openapi > openapi.yaml"
fi

echo
echo "✅ Instructions provided for static OpenAPI generation"
echo
echo "The OpenAPI spec is available at runtime:"
echo "  - JSON: http://localhost:8080/api-docs/openapi.json"
echo "  - UI:   http://localhost:8080/api-docs"
