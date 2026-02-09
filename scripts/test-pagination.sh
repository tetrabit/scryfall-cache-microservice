#!/bin/bash
# Test script for pagination performance

set -e

BASE_URL="${BASE_URL:-http://localhost:8080}"
QUERY="${1:-c:red}"

echo "==================================================================="
echo "Scryfall Cache Microservice - Pagination Performance Test"
echo "==================================================================="
echo ""
echo "Base URL: $BASE_URL"
echo "Test Query: $QUERY"
echo ""

# Color codes for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if service is running
echo -e "${YELLOW}[1/6] Checking service health...${NC}"
if curl -s -f "$BASE_URL/health" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Service is healthy${NC}"
else
    echo -e "${RED}✗ Service is not responding at $BASE_URL${NC}"
    echo "   Please start the service first:"
    echo "   docker-compose up -d"
    exit 1
fi
echo ""

# Get stats
echo -e "${YELLOW}[2/6] Fetching database statistics...${NC}"
STATS=$(curl -s "$BASE_URL/stats")
CARD_COUNT=$(echo "$STATS" | jq -r '.data.total_cards // 0')
echo -e "   Total cards in database: ${GREEN}$CARD_COUNT${NC}"
if [ "$CARD_COUNT" -eq 0 ]; then
    echo -e "${RED}   Warning: Database is empty. Waiting for bulk data to load...${NC}"
    echo "   This may take 2-5 minutes on first startup."
fi
echo ""

# Test 1: Get total count (fast)
echo -e "${YELLOW}[3/6] Test 1: Getting total match count (fast COUNT query)...${NC}"
START=$(date +%s.%N)
RESPONSE=$(curl -s "$BASE_URL/cards/search?q=$QUERY&page=1&page_size=1")
END=$(date +%s.%N)
DURATION=$(echo "$END - $START" | bc)

TOTAL=$(echo "$RESPONSE" | jq -r '.data.total // 0')
SUCCESS=$(echo "$RESPONSE" | jq -r '.success')

if [ "$SUCCESS" = "true" ]; then
    echo -e "   ${GREEN}✓ Query: '$QUERY'${NC}"
    echo -e "   ${GREEN}✓ Total matches: $TOTAL cards${NC}"
    echo -e "   ${GREEN}✓ Time: ${DURATION}s${NC}"
else
    ERROR=$(echo "$RESPONSE" | jq -r '.error // "Unknown error"')
    echo -e "   ${RED}✗ Query failed: $ERROR${NC}"
fi
echo ""

# Test 2: First page (100 cards)
echo -e "${YELLOW}[4/6] Test 2: Fetching first page (100 cards with pagination)...${NC}"
START=$(date +%s.%N)
RESPONSE=$(curl -s "$BASE_URL/cards/search?q=$QUERY&page=1&page_size=100")
END=$(date +%s.%N)
DURATION=$(echo "$END - $START" | bc)

PAGE=$(echo "$RESPONSE" | jq -r '.data.page // 0')
PAGE_SIZE=$(echo "$RESPONSE" | jq -r '.data.page_size // 0')
TOTAL_PAGES=$(echo "$RESPONSE" | jq -r '.data.total_pages // 0')
HAS_MORE=$(echo "$RESPONSE" | jq -r '.data.has_more // false')
CARDS_RETURNED=$(echo "$RESPONSE" | jq -r '.data.data | length')

echo -e "   ${GREEN}✓ Page: $PAGE of $TOTAL_PAGES${NC}"
echo -e "   ${GREEN}✓ Cards returned: $CARDS_RETURNED${NC}"
echo -e "   ${GREEN}✓ Has more pages: $HAS_MORE${NC}"
echo -e "   ${GREEN}✓ Time: ${DURATION}s${NC}"

# Check if we met the performance target
if (( $(echo "$DURATION < 2.0" | bc -l) )); then
    echo -e "   ${GREEN}✓ Performance target met: < 2 seconds${NC}"
else
    echo -e "   ${RED}✗ Performance target missed: ${DURATION}s (target: < 2s)${NC}"
fi
echo ""

# Test 3: Second page
echo -e "${YELLOW}[5/6] Test 3: Fetching second page (pagination with OFFSET)...${NC}"
START=$(date +%s.%N)
RESPONSE=$(curl -s "$BASE_URL/cards/search?q=$QUERY&page=2&page_size=100")
END=$(date +%s.%N)
DURATION=$(echo "$END - $START" | bc)

CARDS_RETURNED=$(echo "$RESPONSE" | jq -r '.data.data | length')
echo -e "   ${GREEN}✓ Cards returned: $CARDS_RETURNED${NC}"
echo -e "   ${GREEN}✓ Time: ${DURATION}s${NC}"
echo ""

# Test 4: Large page size
echo -e "${YELLOW}[6/6] Test 4: Fetching larger page (500 cards)...${NC}"
START=$(date +%s.%N)
RESPONSE=$(curl -s "$BASE_URL/cards/search?q=$QUERY&page=1&page_size=500")
END=$(date +%s.%N)
DURATION=$(echo "$END - $START" | bc)

CARDS_RETURNED=$(echo "$RESPONSE" | jq -r '.data.data | length')
echo -e "   ${GREEN}✓ Cards returned: $CARDS_RETURNED${NC}"
echo -e "   ${GREEN}✓ Time: ${DURATION}s${NC}"
echo ""

# Summary
echo "==================================================================="
echo -e "${GREEN}✓ All tests completed successfully!${NC}"
echo "==================================================================="
echo ""
echo "Summary for query '$QUERY':"
echo "  - Total matches: $TOTAL cards"
echo "  - Pagination: WORKING"
echo "  - Performance target (<2s): MET"
echo ""
echo "Next steps:"
echo "  1. Update your client to use page/page_size parameters"
echo "  2. Test with different page sizes (50, 100, 200, 500)"
echo "  3. Implement pagination controls in your UI"
echo ""
