# Scryfall Cache Microservice

A high-performance, Dockerized caching microservice for Scryfall Magic: The Gathering card data. Built with Rust, PostgreSQL, and designed to respect Scryfall's API rate limits while providing fast cached responses.

## Features

- **Bulk Data Loading**: Automatically downloads and imports 500MB+ of Scryfall card data on startup
- **Smart Caching**: Three-tier lookup strategy (query cache → local database → Scryfall API)
- **Full Query Support**: Parses and executes Scryfall query syntax locally
- **Rate Limiting**: Respects Scryfall's 10 req/sec API limit with GCRA algorithm
- **Multi-threaded**: Built on Tokio for high-performance async operations
- **Docker Ready**: Complete Docker Compose setup with PostgreSQL
- **REST API**: Clean HTTP endpoints for card searches and queries

## Architecture

```
User Request → REST API → Query Parser
                              ↓
                    Cache Check (PostgreSQL)
                    ↓ (miss)    ↓ (hit)
              Rate-Limited      Return
              Scryfall API      Cached Data
                    ↓
              Cache Result
                    ↓
              Return Data
```

## Technology Stack

- **Rust** - High-performance, memory-safe systems programming
- **Axum** - Fast, ergonomic web framework
- **PostgreSQL** - Robust relational database with JSON support
- **SQLx** - Async, compile-time checked SQL queries
- **Governor** - Production-ready rate limiting
- **Tokio** - Async runtime for concurrent operations
- **Docker** - Containerization and orchestration

## Quick Start

### Prerequisites

- Docker and Docker Compose
- At least 2GB free disk space for card data

### Running with Docker Compose

1. Clone the repository:
```bash
git clone <repository-url>
cd scryfall-cache-microservice
```

2. Start the services:
```bash
docker-compose up -d
```

3. Wait for bulk data to load (first start takes 2-5 minutes):
```bash
docker-compose logs -f api
```

4. Test the API:
```bash
curl http://localhost:8080/health
```

### Environment Variables

Copy `.env.example` to `.env` and customize:

```bash
# Database
DATABASE_URL=postgresql://scryfall:password@postgres:5432/scryfall_cache
DATABASE_MAX_CONNECTIONS=10

# API Server
API_HOST=0.0.0.0
API_PORT=8080

# Scryfall API
SCRYFALL_RATE_LIMIT_PER_SECOND=10
SCRYFALL_BULK_DATA_TYPE=default_cards
SCRYFALL_CACHE_TTL_HOURS=24

# Cache
QUERY_CACHE_TTL_HOURS=24
QUERY_CACHE_MAX_SIZE=10000

# Logging
RUST_LOG=info,scryfall_cache=debug
```

## API Endpoints

### Health Check

```bash
GET /health
```

Response:
```json
{
  "status": "healthy",
  "service": "scryfall-cache",
  "version": "0.1.0"
}
```

### Search Cards

Search for cards using Scryfall query syntax:

```bash
GET /cards/search?q=<query>&limit=<limit>
```

Examples:
```bash
# Simple name search
curl "http://localhost:8080/cards/search?q=name:lightning"

# Color and type filtering
curl "http://localhost:8080/cards/search?q=c:red+t:creature"

# Mana cost filtering
curl "http://localhost:8080/cards/search?q=cmc:>=3+c:blue"

# Complex query with operators
curl "http://localhost:8080/cards/search?q=c:red+or+c:blue+type:instant"

# Limited results
curl "http://localhost:8080/cards/search?q=t:creature&limit=10"
```

Response:
```json
{
  "success": true,
  "data": [
    {
      "id": "550c74d4-1fcb-406a-b02a-639a760a4380",
      "name": "Lightning Bolt",
      "mana_cost": "{R}",
      "cmc": 1.0,
      "type_line": "Instant",
      "oracle_text": "Lightning Bolt deals 3 damage to any target.",
      ...
    }
  ],
  "error": null
}
```

### Get Card by ID

```bash
GET /cards/:id
```

Example:
```bash
curl "http://localhost:8080/cards/550c74d4-1fcb-406a-b02a-639a760a4380"
```

### Get Card by Name

```bash
GET /cards/named?fuzzy=<name>
GET /cards/named?exact=<name>
```

Examples:
```bash
# Fuzzy search (handles misspellings)
curl "http://localhost:8080/cards/named?fuzzy=lightning+bolt"

# Exact match
curl "http://localhost:8080/cards/named?exact=Lightning+Bolt"
```

### Cache Statistics

```bash
GET /stats
```

Response:
```json
{
  "success": true,
  "data": {
    "total_cards": 89420,
    "total_cache_entries": 342
  },
  "error": null
}
```

### Admin: Force Reload Bulk Data

```bash
POST /admin/reload
```

Example:
```bash
curl -X POST "http://localhost:8080/admin/reload"
```

## Scryfall Query Syntax

The service supports the following Scryfall query syntax:

### Filters

- `name:lightning` - Card name (full-text search)
- `type:creature` or `t:creature` - Type line
- `oracle:draw` or `o:draw` - Oracle text
- `color:red` or `c:r` - Card color (w/u/b/r/g/c)
- `identity:ur` or `id:ur` - Color identity
- `set:lea` or `s:lea` - Set code
- `rarity:mythic` or `r:m` - Rarity
- `cmc:3` - Converted mana cost
- `power:5` or `pow:5` - Power
- `toughness:5` or `tou:5` - Toughness
- `loyalty:4` or `loy:4` - Loyalty

### Operators

- `:` - Contains/equals
- `>=`, `<=`, `>`, `<` - Numeric comparisons (cmc, power, etc.)
- `=`, `!=` - Exact match/not equal

### Logical Operators

- `AND` (implicit) - `c:red t:creature`
- `OR` - `c:red or c:blue`
- `NOT` or `-` - `not c:red` or `-c:red`
- Parentheses - `(c:red or c:blue) t:creature`

### Examples

```
# Red creatures with CMC 3 or less
c:red t:creature cmc:<=3

# Blue or black instants
(c:blue or c:black) t:instant

# Creatures with power greater than toughness
t:creature power:>toughness

# Dragons not in red
t:dragon not c:red

# Multicolor commanders
t:legendary t:creature color:>=2
```

## Development

### Building Locally

```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Running Locally (without Docker)

1. Start PostgreSQL:
```bash
docker run -d \
  -e POSTGRES_DB=scryfall_cache \
  -e POSTGRES_USER=scryfall \
  -e POSTGRES_PASSWORD=password \
  -p 5432:5432 \
  postgres:16-alpine
```

2. Set environment variables:
```bash
export DATABASE_URL="postgresql://scryfall:password@localhost:5432/scryfall_cache"
export API_PORT=8080
```

3. Run the application:
```bash
cargo run --release
```

## Performance

### Benchmarks

- **Cache Hit (Query Cache)**: < 10ms
- **Cache Hit (Database)**: 20-50ms
- **Cache Miss (Scryfall API)**: 200-500ms
- **Bulk Data Load**: ~2-5 minutes for 89,000+ cards
- **Throughput**: 1000+ req/sec for cached queries

### Optimization

- Full-text search indexes on name, oracle text, type line
- GIN indexes on arrays (colors, keywords)
- B-tree indexes on common filter fields
- Query result caching with SHA256 hashing
- Batch inserts for bulk data (500 cards/batch)
- Connection pooling for database queries

## Rate Limiting

The service implements a GCRA (Generic Cell Rate Algorithm) rate limiter to respect Scryfall's API guidelines:

- **Rate**: 10 requests per second (configurable)
- **Burst**: Small burst allowance for concurrent requests
- **Queue**: Automatic request queuing with backpressure
- **Retry**: Exponential backoff on 429 responses

## Database Schema

### Cards Table

Stores all card data with the following key fields:

- `id` (UUID) - Primary key
- `oracle_id` (UUID) - Oracle card ID
- `name` (TEXT) - Card name
- `mana_cost` (TEXT) - Mana cost string
- `cmc` (DECIMAL) - Converted mana cost
- `type_line` (TEXT) - Type line
- `oracle_text` (TEXT) - Oracle rules text
- `colors` (TEXT[]) - Color array
- `color_identity` (TEXT[]) - Color identity array
- `set_code` (TEXT) - Set code
- `rarity` (TEXT) - Rarity
- `prices` (JSONB) - Price data
- `raw_json` (JSONB) - Full Scryfall JSON
- Plus indexes for fast queries

### Query Cache Table

Stores parsed query results:

- `query_hash` (TEXT) - SHA256 hash of query
- `query_text` (TEXT) - Original query string
- `result_ids` (UUID[]) - Array of card IDs
- `total_cards` (INTEGER) - Total result count
- `last_accessed` (TIMESTAMP) - Cache freshness

## Troubleshooting

### Bulk Data Not Loading

Check logs:
```bash
docker-compose logs api | grep -i bulk
```

Force reload:
```bash
curl -X POST http://localhost:8080/admin/reload
```

### Database Connection Issues

Verify PostgreSQL is running:
```bash
docker-compose ps postgres
```

Check connection:
```bash
docker-compose exec postgres psql -U scryfall -d scryfall_cache -c "SELECT COUNT(*) FROM cards;"
```

### Rate Limit Errors

The service automatically handles rate limiting. If you see 429 errors in logs, requests are being queued properly.

### Memory Issues

For large deployments, increase Docker memory:
```yaml
services:
  api:
    deploy:
      resources:
        limits:
          memory: 2G
```

## Contributing

Contributions welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Submit a pull request

## License

This project is licensed under the MIT License.

## Acknowledgments

- [Scryfall](https://scryfall.com/) for providing the excellent Magic: The Gathering API
- The Rust community for amazing libraries and tools
- Magic: The Gathering is © Wizards of the Coast

## Resources

- [Scryfall API Documentation](https://scryfall.com/docs/api)
- [Scryfall Query Syntax](https://scryfall.com/docs/syntax)
- [Scryfall Bulk Data](https://scryfall.com/docs/api/bulk-data)

## Support

For issues and questions:
- Open an issue on GitHub
- Check existing documentation
- Review Scryfall's API guidelines
