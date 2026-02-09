# Pagination Quick Reference

## TL;DR - Already Implemented ✅

Pagination is **already working**. Just add these parameters to your API calls:

```bash
curl "http://localhost:8080/cards/search?q=c:red&page=1&page_size=100"
```

## API Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `q` | string | **required** | Scryfall query (e.g., "c:red", "t:creature") |
| `page` | integer | 1 | Page number (starts at 1) |
| `page_size` | integer | 100 | Results per page (max: 1000) |

## Response Format

```json
{
  "success": true,
  "data": {
    "data": [...],           // Array of Card objects
    "total": 6704,           // Total matching cards
    "page": 1,               // Current page number
    "page_size": 100,        // Cards per page
    "total_pages": 68,       // Total pages available
    "has_more": true         // More pages available?
  }
}
```

## Quick Examples

### Example 1: First page of red cards
```bash
curl "http://localhost:8080/cards/search?q=c:red&page=1&page_size=100"
```

### Example 2: Get total count only
```bash
curl "http://localhost:8080/cards/search?q=c:red&page=1&page_size=1" | jq '.data.total'
# Output: 6704
```

### Example 3: Get second page
```bash
curl "http://localhost:8080/cards/search?q=c:red&page=2&page_size=100"
```

### Example 4: Larger page size
```bash
curl "http://localhost:8080/cards/search?q=c:red&page=1&page_size=500"
```

## JavaScript/TypeScript Integration

```typescript
// Fetch paginated results
async function searchCards(query: string, page: number = 1, pageSize: number = 100) {
  const url = new URL('http://localhost:8080/cards/search');
  url.searchParams.set('q', query);
  url.searchParams.set('page', page.toString());
  url.searchParams.set('page_size', pageSize.toString());
  
  const response = await fetch(url.toString());
  const result = await response.json();
  
  if (result.success) {
    return {
      cards: result.data.data,
      total: result.data.total,
      page: result.data.page,
      totalPages: result.data.total_pages,
      hasMore: result.data.has_more
    };
  }
  
  throw new Error(result.error);
}

// Usage
const { cards, total, hasMore } = await searchCards('c:red', 1, 100);
console.log(`Found ${total} cards, showing ${cards.length}`);
```

## Performance Expectations

| Operation | Time | Notes |
|-----------|------|-------|
| Count query | ~0.1-0.5s | Fast SELECT COUNT(*) |
| First page (100 cards) | ~0.5-1.5s | LIMIT 100 OFFSET 0 |
| Subsequent pages | ~0.3-1s | LIMIT 100 OFFSET N |
| Large page (500 cards) | ~1-2s | More data transfer |

**Target:** < 2 seconds for any page ✅

## Common Queries

```bash
# Red cards
curl "http://localhost:8080/cards/search?q=c:red&page=1&page_size=100"

# Red creatures
curl "http://localhost:8080/cards/search?q=c:red+t:creature&page=1&page_size=100"

# Cheap red instants (CMC ≤ 2)
curl "http://localhost:8080/cards/search?q=c:red+t:instant+cmc:<=2&page=1&page_size=100"

# Dragons
curl "http://localhost:8080/cards/search?q=t:dragon&page=1&page_size=100"

# Mythic rares from a set
curl "http://localhost:8080/cards/search?q=s:khm+r:mythic&page=1&page_size=100"
```

## Testing

Run the test script:
```bash
cd ~/projects/scryfall-cache-microservice
./scripts/test-pagination.sh "c:red"
```

Or test manually:
```bash
# Should complete in < 2 seconds
time curl "http://localhost:8080/cards/search?q=c:red&page=1&page_size=100" > /dev/null
```

## Troubleshooting

### Problem: Still getting 41-second responses

**Cause:** Not using pagination parameters  
**Solution:** Add `page` and `page_size` to your API calls

### Problem: Service not responding

**Cause:** Microservice not running  
**Solution:**
```bash
docker-compose up -d
docker-compose logs -f api
```

### Problem: Database is empty

**Cause:** Bulk data not loaded yet  
**Solution:** Wait 2-5 minutes on first startup for bulk import

### Problem: Slow queries

**Cause:** Missing indexes  
**Solution:** Check migrations ran:
```bash
docker-compose exec postgres psql -U scryfall -d scryfall_cache -c "
  SELECT indexname FROM pg_indexes WHERE tablename = 'cards';
"
```

## Architecture

```
API Request
  ↓
  q=c:red&page=1&page_size=100
  ↓
QueryExecutor.execute_paginated()
  ↓
  SELECT COUNT(*) FROM cards WHERE ...  -- Fast count
  ↓
  SELECT * FROM cards WHERE ... LIMIT 100 OFFSET 0  -- Paginated fetch
  ↓
Response: { data: [100 cards], total: 6704, page: 1, ... }
```

## Implementation Files

- **API Handler:** `src/api/handlers.rs` (lines 181-220)
- **Cache Manager:** `src/cache/manager.rs` (lines 113-162)
- **Query Executor:** `src/query/executor.rs` (lines 90-139)
- **PostgreSQL Backend:** `src/db/postgres/queries.rs` (lines 248-279)
- **SQLite Backend:** `src/db/sqlite/queries.rs` (lines 311-344)

## For More Details

See: `PAGINATION_IMPLEMENTATION_STATUS.md`
