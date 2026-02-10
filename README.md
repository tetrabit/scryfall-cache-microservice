# Scryfall Cache Microservice

A high-performance, caching microservice for Scryfall Magic: The Gathering card data. Built with Rust and **supports both PostgreSQL and SQLite backends**. Designed to respect Scryfall's API rate limits while providing fast cached responses.

## Features

- **Dual Database Backends**: PostgreSQL (production) or SQLite (Electron/embedded)
- **Optional Redis Cache**: Ultra-fast in-memory caching layer (1-5ms response time)
- **Bulk Data Loading**: Automatically downloads and imports 500MB+ of Scryfall card data on startup
- **Smart Caching**: Four-tier lookup strategy (Redis → query cache → local database → Scryfall API)
- **Full Query Support**: Parses and executes Scryfall query syntax locally
- **Rate Limiting**: Respects Scryfall's 10 req/sec API limit with GCRA algorithm
- **Multi-threaded**: Built on Tokio for high-performance async operations
- **Docker Ready**: Complete Docker Compose setup with PostgreSQL and Redis
- **Low Memory**: SQLite backend uses <100MB RAM (vs PostgreSQL's 500MB)
- **REST API**: Clean HTTP endpoints for card searches and queries

## Database Backends

### PostgreSQL (Default)
- **Use case**: Production servers, Docker deployments
- **Memory**: ~500MB RAM
- **Features**: Full-text search, advanced indexing, high concurrency
- **Setup**: Requires PostgreSQL server

### SQLite
- **Use case**: Electron apps, embedded systems, development
- **Memory**: **<100MB RAM** ✅
- **Features**: Zero configuration, single file, bundled with binary
- **Setup**: Auto-creates database file

See [SQLITE_BACKEND.md](./SQLITE_BACKEND.md) for detailed comparison and usage.

## Architecture

```
User Request → REST API → Query Parser
                              ↓
                    Redis Cache Check (optional, 1-5ms)
                    ↓ (miss)    ↓ (hit)
              Query Cache Check (PostgreSQL, 20-50ms)
              ↓ (miss)    ↓ (hit)
        Rate-Limited      Return Cached Data
        Scryfall API
        (200-500ms)
              ↓
        Cache Result (Redis + DB)
              ↓
        Return Data
```

**Performance tiers:**
1. **Redis** (optional): <5ms - Hot query results and frequently accessed cards
2. **PostgreSQL/SQLite**: 20-50ms - All cards and query cache
3. **Scryfall API**: 200-500ms - Fallback for missing data

## Redis Cache Layer (Optional)

The Redis cache layer provides sub-5ms response times for frequently accessed data. It's **optional** and disabled by default.

### When to Enable Redis

- **High traffic**: Multiple concurrent users performing similar queries
- **Hot data**: Frequently accessed cards or popular queries (e.g., top competitive decks)
- **Performance critical**: Applications requiring <10ms p99 latency

### When to Skip Redis

- **Low traffic**: Single user or infrequent requests
- **Development**: Local development environments
- **Cost sensitive**: Minimal infra deploy (SQLite is sufficient for small sites)

### Enabling Redis

**With Docker Compose:**
```bash
# Set REDIS_ENABLED=true in .env or export it
export REDIS_ENABLED=true
docker-compose up -d
```

**Standalone:**
```bash
# Start Redis
docker run -d -p 6379:6379 redis:7-alpine

# Enable in application
export REDIS_ENABLED=true
export REDIS_URL=redis://localhost:6379
cargo run --release --features redis_cache
```

**Build Requirements:**
- Redis feature must be enabled at compile time: `--features redis_cache`
- Or use default features which include both postgres and redis_cache

### Redis Configuration

```bash
REDIS_ENABLED=true                 # Enable/disable Redis cache
REDIS_URL=redis://localhost:6379   # Redis connection URL
REDIS_TTL_SECONDS=3600             # Cache TTL (1 hour default)
REDIS_MAX_VALUE_SIZE_MB=10         # Skip caching values larger than this
```

### What Gets Cached in Redis

- **Query results**: Search query card IDs (fastest lookup)
- **Individual cards**: Frequently accessed cards by ID
- **Autocomplete**: Name prefix results (10-minute TTL)

### Fallback Behavior

If Redis is unreachable, the service automatically falls back to PostgreSQL/SQLite without errors. This ensures high availability even if Redis goes down.

## Scaling Notes (Scale-Ready, Not Scaled)

This service is intended to stay simple for a single low-traffic website, while keeping a clean path to scale later.

- **Stateless API**: persistent state lives in the database; caching is stored in the database (not in-process).
- **Multiple instances**: you can run more than one API process against the same DB without correctness changes.
- **Background refresh**: if you run multiple instances, consider setting `BULK_REFRESH_ENABLED=false` on all but one instance to avoid redundant bulk downloads/imports.
- **Health endpoints**: use `/health/live` for liveness and `/health/ready` for readiness-based routing.

When to scale further:
- Add horizontal scaling/LB only when you need higher availability or zero-downtime deploys.
- Add read replicas only once the DB becomes a read bottleneck after query/index tuning.

## Technology Stack

- **Rust** - High-performance, memory-safe systems programming
- **Axum** - Fast, ergonomic web framework
- **PostgreSQL** / **SQLite** - Dual backend support via trait abstraction
- **Redis** (optional) - In-memory cache for sub-5ms response times
- **SQLx** / **rusqlite** - Async database drivers
- **Governor** - Production-ready rate limiting
- **Tokio** - Async runtime for concurrent operations
- **Docker** - Containerization and orchestration

## Quick Start

### PostgreSQL (Docker - Recommended for Production)

#### Prerequisites

- Docker and Docker Compose
- At least 2GB free disk space for card data

#### Running with Docker Compose

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

### SQLite (Standalone - Recommended for Electron)

#### Prerequisites

- Rust toolchain (1.70+)

#### Building and Running

1. Clone and build:
```bash
git clone <repository-url>
cd scryfall-cache-microservice

# Build with SQLite backend
cargo build --release --no-default-features --features sqlite
```

2. Run the service:
```bash
# Database will be auto-created at ./data/scryfall-cache.db
export SQLITE_PATH="./data/scryfall-cache.db"
export PORT=8080
./target/release/scryfall-cache
```

3. Test:
```bash
curl http://localhost:8080/health
```

**Memory usage**: ~45-80MB (vs PostgreSQL's 500MB)

See [SQLITE_BACKEND.md](./SQLITE_BACKEND.md) for Electron integration guide.

### Environment Variables

#### PostgreSQL Configuration

Copy `.env.example` to `.env` and customize:

```bash
# Database
DATABASE_URL=postgresql://scryfall:password@postgres:5432/scryfall_cache
DATABASE_MAX_CONNECTIONS=10
DATABASE_MIN_CONNECTIONS=0
DATABASE_ACQUIRE_TIMEOUT_MS=30000
DATABASE_IDLE_TIMEOUT_SECONDS=600
DATABASE_MAX_LIFETIME_SECONDS=1800

# API Server
API_HOST=0.0.0.0
API_PORT=8080
INSTANCE_ID=api-1

# Scryfall API
SCRYFALL_API_BASE_URL=https://api.scryfall.com
SCRYFALL_RATE_LIMIT_PER_SECOND=10

# Background jobs
# If you run multiple API instances, consider disabling refresh on all but one instance.
BULK_REFRESH_ENABLED=true
BULK_REFRESH_INTERVAL_HOURS=720
```

#### SQLite Configuration

```bash
# Database file path (optional, defaults to ./data/scryfall-cache.db)
SQLITE_PATH=/path/to/database.db

# API Server
PORT=8080
HOST=127.0.0.1
```

## Building from Source

### PostgreSQL Build

```bash
# Default build (PostgreSQL)
cargo build --release

# Explicit PostgreSQL
cargo build --release --features postgres
```

### SQLite Build

```bash
# SQLite only
cargo build --release --no-default-features --features sqlite
```

Binary will be at `target/release/scryfall-cache` (~19MB stripped).

## Development

Example local environment:

```bash
API_PORT=8080
INSTANCE_ID=api-1

# Scryfall API
SCRYFALL_RATE_LIMIT_PER_SECOND=10
SCRYFALL_BULK_DATA_TYPE=default_cards
SCRYFALL_CACHE_TTL_HOURS=720  # 30 days - bulk data refreshed monthly max

# Cache
QUERY_CACHE_TTL_HOURS=24
QUERY_CACHE_MAX_SIZE=10000

# Logging
RUST_LOG=info,scryfall_cache=debug

# Background jobs
BULK_REFRESH_ENABLED=true
BULK_REFRESH_INTERVAL_HOURS=720
```

## Local Checks

Run the standard local checks (backend + admin-panel) with:

```bash
./scripts/ci.sh
```

## API Endpoints

### Health Checks

```bash
GET /health
GET /health/live
GET /health/ready
```

`/health` and `/health/live` are liveness-style endpoints (no dependency checks). Use `/health/ready` for readiness (returns `503` if dependencies are unavailable).

Response (example):
```json
{
  "status": "ready",
  "service": "scryfall-cache",
  "version": "0.1.0",
  "instance_id": "api-1",
  "checks": {
    "database": "ok"
  }
}
```

You can set `INSTANCE_ID` (or rely on `HOSTNAME`) to help debug which instance served a request.

## Admin Panel

There is a lightweight React admin UI in `admin-panel/` that reads backend JSON endpoints and links out to `/metrics` and `/api-docs`.

Dev:

```bash
cd admin-panel
npm install
npm run dev
```

Then open `http://localhost:5173/admin/` (the Vite app is configured with `base: /admin/`).

Prod:

```bash
cd admin-panel
npm ci
npm run build
```

The backend serves the built assets from `admin-panel/dist` at `GET /admin`.

Backend endpoints used by the UI:
- `GET /api/admin/stats/overview`
- `POST /admin/reload`

Note: authentication for admin endpoints is not implemented yet; treat these as trusted-network only until API key auth exists.

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

### Batch Get Cards by ID

Fetch many cards in a single request (significantly faster than N sequential calls).

```bash
POST /cards/batch
```

Example:
```bash
curl -X POST "http://localhost:8080/cards/batch" \
  -H "content-type: application/json" \
  -d '{
    "ids": [
      "550c74d4-1fcb-406a-b02a-639a760a4380",
      "00000000-0000-0000-0000-000000000000"
    ],
    "fetch_missing": false
  }'
```

Response (example):
```json
{
  "success": true,
  "data": {
    "cards": [
      { "id": "550c74d4-1fcb-406a-b02a-639a760a4380", "name": "Lightning Bolt", "...": "..." }
    ],
    "missing_ids": ["00000000-0000-0000-0000-000000000000"]
  },
  "error": null
}
```

Set `BATCH_MAX_IDS` to limit the maximum number of IDs accepted (default: 1000).

### Batch Get Cards by Name

Fetch multiple cards by name in one request.

```bash
POST /cards/named/batch
```

Example:
```bash
curl -X POST "http://localhost:8080/cards/named/batch" \
  -H "content-type: application/json" \
  -d '{
    "names": ["Lightning Bolt", "Sol Ring"],
    "fuzzy": true
  }'
```

Set `BATCH_MAX_NAMES` to limit the maximum number of names accepted (default: 50).

### Batch Execute Queries

Execute multiple search queries in a single request (returns per-query results).

```bash
POST /queries/batch
```

Example:
```bash
curl -X POST "http://localhost:8080/queries/batch" \
  -H "content-type: application/json" \
  -d '{
    "queries": [
      { "id": "q1", "query": "c:r", "page": 1, "page_size": 10 },
      { "id": "q2", "query": "t:instant c:u", "page": 1, "page_size": 10 }
    ]
  }'
```

Set `BATCH_MAX_QUERIES` to limit the maximum number of queries accepted (default: 10).

You can set `BATCH_PARALLELISM` to control how many batch items are processed concurrently (default: 4).

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

### Autocomplete Card Names

Get card name suggestions based on a prefix (case-insensitive). Returns up to 20 matching card names, sorted alphabetically. Minimum 2 characters required.

```bash
GET /cards/autocomplete?q=<prefix>
```

Examples:
```bash
# Get cards starting with "light"
curl "http://localhost:8080/cards/autocomplete?q=light"

# Get cards starting with "sol r"
curl "http://localhost:8080/cards/autocomplete?q=sol+r"

# Get cards starting with "force"
curl "http://localhost:8080/cards/autocomplete?q=force"
```

Response (Scryfall catalog format):
```json
{
  "object": "catalog",
  "data": [
    "Light Up the Night",
    "Light Up the Stage",
    "Lightning Axe",
    "Lightning Bolt",
    "Lightning Helix",
    "Lightning Strike",
    "Lightspeed"
  ]
}
```

**Performance**: Optimized with database indexes for <100ms response time. Perfect for search-as-you-type interfaces.

**Cache**: Results are not cached as autocomplete queries are typically diverse and short-lived.

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
