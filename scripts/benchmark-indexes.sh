#!/bin/bash
# Phase 2 Database Index Performance Benchmarking Script
# Tests query performance before and after index optimization
# Usage: ./scripts/benchmark-indexes.sh [postgres|sqlite]

set -e

BACKEND=${1:-postgres}
BASE_URL="http://localhost:8080"

echo "=================================================="
echo "Phase 2: Database Index Performance Benchmark"
echo "Backend: $BACKEND"
echo "=================================================="
echo ""

# Color codes for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to run a benchmark query
benchmark_query() {
    local query=$1
    local description=$2
    local target_time=$3
    
    echo -n "Testing: $description ... "
    
    # Run query 3 times and take average
    local total_time=0
    for i in {1..3}; do
        local start=$(date +%s%3N)
        curl -s "$BASE_URL/api/v1/cards/search?q=$(echo $query | jq -sRr @uri)&limit=100" > /dev/null
        local end=$(date +%s%3N)
        local duration=$((end - start))
        total_time=$((total_time + duration))
    done
    
    local avg_time=$((total_time / 3))
    local seconds=$(echo "scale=2; $avg_time / 1000" | bc)
    
    if (( avg_time <= target_time )); then
        echo -e "${GREEN}✓ ${seconds}s${NC} (target: <$(echo "scale=2; $target_time / 1000" | bc)s)"
    else
        echo -e "${YELLOW}⚠ ${seconds}s${NC} (target: <$(echo "scale=2; $target_time / 1000" | bc)s)"
    fi
}

# Check if service is running
if ! curl -s "$BASE_URL/health" > /dev/null 2>&1; then
    echo "Error: Service not running at $BASE_URL"
    echo "Start service with: cargo run --release"
    exit 1
fi

echo "Phase 2 Success Criteria:"
echo "  - Broad queries (c:red): <1000ms"
echo "  - Medium queries (t:creature c:red): <500ms"
echo "  - Complex queries: <1000ms"
echo ""
echo "Running benchmarks..."
echo ""

# Broad queries (should benefit most from indexes)
echo "=== Broad Queries ==="
benchmark_query "c:red" "Single color query (c:red)" 1000
benchmark_query "c:blue" "Single color query (c:blue)" 1000
benchmark_query "cmc<=3" "CMC range query (cmc<=3)" 1000
benchmark_query "t:creature" "Type query (t:creature)" 1000

echo ""
echo "=== Medium Specificity Queries ==="
benchmark_query "c:red t:creature" "Color + Type (c:red t:creature)" 500
benchmark_query "c:blue cmc<=3" "Color + CMC (c:blue cmc<=3)" 500
benchmark_query "t:instant c:red" "Type + Color (t:instant c:red)" 500
benchmark_query "set:mid r:rare" "Set + Rarity (set:mid r:rare)" 500

echo ""
echo "=== Complex Queries ==="
benchmark_query "c:red t:creature cmc<=3" "Multi-filter (color+type+cmc)" 1000
benchmark_query "t:legendary c:red c:white" "Multi-color legendary" 1000
benchmark_query "set:mid (t:creature OR t:instant)" "Set with OR logic" 1000

echo ""
echo "=== Narrow Queries (regression check) ==="
benchmark_query "Lightning Bolt" "Name search (Lightning Bolt)" 100
benchmark_query "set:mid number:123" "Specific card (set+number)" 100

echo ""
echo "=================================================="
echo "Benchmark complete!"
echo "=================================================="
echo ""
echo "Next steps:"
echo "  1. Review results against success criteria"
echo "  2. If targets not met, run EXPLAIN ANALYZE on slow queries"
echo "  3. Consider additional composite indexes for slow patterns"
echo ""
