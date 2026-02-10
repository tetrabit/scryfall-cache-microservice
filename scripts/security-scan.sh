#!/bin/bash
# Security scanning script for Scryfall Cache Microservice

set -e

echo "=== Scryfall Cache Security Scan ==="
echo

# Check if required tools are installed
check_tool() {
    if ! command -v $1 &> /dev/null; then
        echo "❌ $1 not found. Install with: $2"
        return 1
    else
        echo "✅ $1 found"
        return 0
    fi
}

echo "Checking for security tools..."
check_tool "cargo-audit" "cargo install cargo-audit" || MISSING_TOOLS=1
check_tool "cargo-deny" "cargo install cargo-deny" || MISSING_TOOLS=1
check_tool "cargo-clippy" "rustup component add clippy" || MISSING_TOOLS=1

if [ "$MISSING_TOOLS" = "1" ]; then
    echo
    echo "Some tools are missing. Install them and run again."
    exit 1
fi

echo
echo "=== Running Dependency Audit ==="
echo "Checking for known vulnerabilities..."
cargo audit || echo "⚠️  Vulnerabilities found"

echo
echo "=== Running License Check ==="
echo "Checking dependency licenses..."
cargo deny check licenses || echo "⚠️  License issues found"

echo
echo "=== Running Advisory Check ==="
echo "Checking security advisories..."
cargo deny check advisories || echo "⚠️  Advisory issues found"

echo
echo "=== Running Ban Check ==="
echo "Checking for banned dependencies..."
cargo deny check bans || echo "⚠️  Banned dependencies found"

echo
echo "=== Running Clippy (Security Lints) ==="
echo "Running static analysis..."
cargo clippy -- -D warnings \
    -D clippy::all \
    -W clippy::pedantic \
    -W clippy::nursery \
    || echo "⚠️  Clippy issues found"

echo
echo "=== Security Scan Complete ==="
