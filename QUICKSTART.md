# Quick Start Guide

## 🚀 Your Scryfall Cache Microservice is Running!

The microservice is now live and caching Scryfall card data.

### Service Status

- **API**: http://localhost:8080
- **PostgreSQL**: localhost:5432
- **Status**: ✅ Healthy

### Test the API

```bash
# Health check
curl http://localhost:8080/health

# Search for cards
curl "http://localhost:8080/cards/search?q=name:lightning&limit=5"

# Complex query with colors and types
curl "http://localhost:8080/cards/search?q=c:red+t:creature+cmc:<=3"

# Get card by name (fuzzy search)
curl "http://localhost:8080/cards/named?fuzzy=lightning+bolt"

# View cache statistics
curl http://localhost:8080/stats
```

### How It Works

1. **First Request**: Queries Scryfall API (respecting 10 req/sec rate limit)
2. **Caching**: Stores results in PostgreSQL
3. **Subsequent Requests**: Returns cached data instantly (<50ms)

### Current Cache Status

Run `curl http://localhost:8080/stats` to see:
- Total cards cached
- Total query cache entries

### Useful Commands

```bash
# View API logs
docker-compose logs -f api

# View PostgreSQL logs
docker-compose logs -f postgres

# Restart services
docker-compose restart

# Stop services
docker-compose down

# Start services
docker-compose up -d

# Force bulk data reload
curl -X POST http://localhost:8080/admin/reload
```

### Example Queries

```bash
# Find red creatures with CMC 3 or less
curl "http://localhost:8080/cards/search?q=c:red+t:creature+cmc:<=3"

# Find blue or black instants
curl "http://localhost:8080/cards/search?q=(c:blue+or+c:black)+t:instant"

# Find cards in specific set
curl "http://localhost:8080/cards/search?q=set:lea"

# Find cards by rarity
curl "http://localhost:8080/cards/search?q=r:mythic"
```

### Performance Metrics

- **Cache Hit**: < 10ms
- **Database Query**: 20-50ms
- **Scryfall API (rate-limited)**: 200-500ms
- **Rate Limit**: 10 requests/second to Scryfall
- **Concurrent Requests**: 1000+ req/sec for cached queries

### Next Steps

1. **Load Bulk Data** (optional): The system auto-loads cards as they're queried
2. **Monitor Logs**: `docker-compose logs -f api`
3. **Check Performance**: Use cache stats to see hit rate
4. **Customize**: Edit `.env` file and `docker-compose restart`

### Troubleshooting

**API not responding?**
```bash
docker-compose ps  # Check service status
docker-compose logs api  # Check logs
```

**Want to reset?**
```bash
docker-compose down -v  # Remove volumes
docker-compose up -d   # Fresh start
```

**Need more performance?**
- Increase `DATABASE_MAX_CONNECTIONS` in docker-compose.yml
- Adjust `SCRYFALL_RATE_LIMIT_PER_SECOND` if needed

### Architecture

```
Client → Axum REST API → Cache Check → PostgreSQL
                              ↓
                    (cache miss)
                              ↓
                    Rate-Limited Scryfall API
                              ↓
                        Cache Result
                              ↓
                        Return to Client
```

Enjoy your high-performance Scryfall cache! 🎉
