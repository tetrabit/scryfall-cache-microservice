#!/bin/bash
# Memory test for SQLite backend
# This script tests memory usage of the microservice with SQLite

set -e

echo "=== Scryfall Cache Microservice - SQLite Memory Test ==="
echo ""

# Build SQLite version
echo "Building SQLite version..."
cargo build --release --no-default-features --features sqlite
echo "✓ Build complete"
echo ""

# Set up environment
export SQLITE_PATH="./test-data/test.db"
export PORT=8080
export HOST="127.0.0.1"

# Clean up any existing test data
rm -rf ./test-data
mkdir -p ./test-data

echo "Starting microservice with SQLite backend..."
echo "Database: $SQLITE_PATH"
echo ""

# Start the service in background
./target/release/scryfall-cache &
PID=$!

# Wait for startup
sleep 3

echo "Service started (PID: $PID)"
echo ""

# Get initial memory
INITIAL_MEM=$(ps -o rss= -p $PID)
echo "Initial memory: $((INITIAL_MEM / 1024)) MB"

# Test basic operations
echo ""
echo "Running basic health check..."
curl -s http://localhost:8080/health > /dev/null && echo "✓ Health check passed" || echo "✗ Health check failed"

# Check memory after startup
STARTUP_MEM=$(ps -o rss= -p $PID)
echo "Memory after startup: $((STARTUP_MEM / 1024)) MB"

# Make some test requests
echo ""
echo "Testing card search (will trigger Scryfall API)..."
curl -s "http://localhost:8080/api/v1/search?q=lightning+bolt&limit=10" > /dev/null && echo "✓ Search completed" || echo "✗ Search failed"

sleep 2

# Check final memory
FINAL_MEM=$(ps -o rss= -p $PID)
echo "Memory after search: $((FINAL_MEM / 1024)) MB"

# Calculate overhead
OVERHEAD=$((FINAL_MEM - INITIAL_MEM))
echo ""
echo "=== RESULTS ==="
echo "Memory overhead: $((OVERHEAD / 1024)) MB"
echo "Total memory usage: $((FINAL_MEM / 1024)) MB"
echo ""

if [ $((FINAL_MEM / 1024)) -lt 100 ]; then
    echo "✅ SUCCESS: Memory usage is under 100MB target!"
else
    echo "⚠️  WARNING: Memory usage exceeds 100MB target"
fi

# Cleanup
kill $PID 2>/dev/null || true
sleep 1
rm -rf ./test-data

echo ""
echo "Test complete!"
