#!/bin/bash
# Phase 2 Database Index Performance Benchmarking Script (Enhanced)
# Tests query performance with detailed timing and EXPLAIN ANALYZE support
# Usage: ./scripts/benchmark-indexes-v2.sh [postgres|sqlite] [--explain]

set -e

BACKEND=${1:-postgres}
BASE_URL="http://localhost:8080"
EXPLAIN_MODE=false

if [[ "$2" == "--explain" ]] || [[ "$1" == "--explain" ]]; then
    EXPLAIN_MODE=true
fi

echo "=================================================="
echo "Phase 2: Database Index Performance Benchmark v2"
echo "Backend: $BACKEND"
echo "Explain Mode: $EXPLAIN_MODE"
echo "=================================================="
echo ""

# Color codes for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to URL encode (simple version)
urlencode() {
    local string="$1"
    echo "$string" | python3 -c "import sys; import urllib.parse; print(urllib.parse.quote(sys.stdin.read().strip()))"
}

# Function to run a benchmark query
benchmark_query() {
    local query=$1
    local description=$2
    local target_ms=$3
    
    echo -n "Testing: $description ... "
    
    # URL encode the query
    local encoded_query=$(urlencode "$query")
    
    # Run query 3 times and collect timings
    local times=()
    for i in {1..3}; do
        local start=$(date +%s%N)
        local response=$(curl -s -w "\n%{http_code}" "$BASE_URL/api/v1/cards/search?q=${encoded_query}&limit=100")
        local end=$(date +%s%N)
        
        local http_code=$(echo "$response" | tail -n1)
        if [[ "$http_code" != "200" ]]; then
            echo -e "${RED}✗ HTTP $http_code${NC}"
            if [[ "$EXPLAIN_MODE" == true ]]; then
                echo "$response" | head -n -1
            fi
            return 1
        fi
        
        local duration_ns=$((end - start))
        local duration_ms=$((duration_ns / 1000000))
        times+=($duration_ms)
    done
    
    # Calculate average
    local sum=0
    for t in "${times[@]}"; do
        sum=$((sum + t))
    done
    local avg_ms=$((sum / 3))
    
    # Format output
    if (( avg_ms <= target_ms )); then
        echo -e "${GREEN}✓ ${avg_ms}ms${NC} (target: <${target_ms}ms) [${times[0]}ms, ${times[1]}ms, ${times[2]}ms]"
    elif (( avg_ms <= target_ms * 2 )); then
        echo -e "${YELLOW}⚠ ${avg_ms}ms${NC} (target: <${target_ms}ms) [${times[0]}ms, ${times[1]}ms, ${times[2]}ms]"
    else
        echo -e "${RED}✗ ${avg_ms}ms${NC} (target: <${target_ms}ms) [${times[0]}ms, ${times[1]}ms, ${times[2]}ms]"
    fi
}

# Function to show EXPLAIN ANALYZE for a query (PostgreSQL only)
explain_query() {
    local query=$1
    
    if [[ "$BACKEND" != "postgres" ]]; then
        echo "EXPLAIN ANALYZE only available for PostgreSQL backend"
        return
    fi
    
    echo -e "\n${BLUE}EXPLAIN ANALYZE for: $query${NC}"
    
    # This would require database access - for now, just show the query
    # In a real implementation, you'd run EXPLAIN ANALYZE via psql
    echo "Run this manually:"
    echo "  docker exec scryfall-cache-postgres psql -U scryfall -d scryfall_cache -c \"EXPLAIN ANALYZE SELECT * FROM cards WHERE ...\""
}

# Check if service is running
if ! curl -s "$BASE_URL/health" > /dev/null 2>&1; then
    echo -e "${RED}Error: Service not running at $BASE_URL${NC}"
    echo "Start service with: cargo run --release"
    exit 1
fi

echo "Phase 2 Success Criteria:"
echo "  - Broad queries (c:red): <1000ms"
echo "  - Medium queries (t:creature c:red): <500ms"
echo "  - Complex queries: <1000ms"
echo "  - Narrow queries (name): <100ms"
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
benchmark_query "t:legendary c:red" "Legendary red permanents" 1000

echo ""
echo "=== Narrow Queries (regression check) ==="
benchmark_query "Lightning Bolt" "Name search (Lightning Bolt)" 100
benchmark_query "!\"Black Lotus\"" "Exact name search (Black Lotus)" 100

echo ""
echo "=================================================="
echo "Benchmark complete!"
echo "=================================================="
echo ""

# Calculate database size
if [[ "$BACKEND" == "postgres" ]]; then
    echo "=== Database Statistics (PostgreSQL) ==="
    docker exec scryfall-cache-postgres psql -U scryfall -d scryfall_cache -t -c "
        SELECT 
            'Total cards: ' || count(*) 
        FROM cards;
    " 2>/dev/null || echo "Could not retrieve card count"
    
    docker exec scryfall-cache-postgres psql -U scryfall -d scryfall_cache -t -c "
        SELECT 
            'Database size: ' || pg_size_pretty(pg_database_size('scryfall_cache'))
        ;
    " 2>/dev/null || echo "Could not retrieve database size"
    
    docker exec scryfall-cache-postgres psql -U scryfall -d scryfall_cache -t -c "
        SELECT 
            'Cards table: ' || pg_size_pretty(pg_total_relation_size('cards'))
        ;
    " 2>/dev/null || echo "Could not retrieve table size"
    
    docker exec scryfall-cache-postgres psql -U scryfall -d scryfall_cache -t -c "
        SELECT 
            'Indexes: ' || count(*) 
        FROM pg_indexes 
        WHERE tablename = 'cards';
    " 2>/dev/null || echo "Could not retrieve index count"
    echo ""
fi

echo "Next steps:"
echo "  1. Review results against success criteria"
echo "  2. If targets not met, run with --explain flag"
echo "  3. Check query execution plans for slow queries"
echo "  4. Consider additional composite indexes if needed"
echo ""
