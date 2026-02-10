# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A high-performance caching microservice for Scryfall Magic: The Gathering card data. Built with Rust, supporting both PostgreSQL (production) and SQLite (embedded/Electron) backends. The service parses and executes Scryfall query syntax locally to minimize API calls while respecting rate limits.

## Build and Test Commands

### Standard Development Workflow

```bash
# Run all checks (format, test, admin-panel lint/build)
./scripts/ci.sh

# Format check
cargo fmt --check

# Run tests
cargo test

# Build (PostgreSQL - default)
cargo build --release

# Build (SQLite - for Electron/embedded)
cargo build --release --no-default-features --features sqlite
```

### Running Locally

**PostgreSQL (Docker - default):**
```bash
docker-compose up -d
docker-compose logs -f api  # Watch startup + bulk data load
```

**PostgreSQL (standalone):**
```bash
# Start PostgreSQL manually
docker run -d -e POSTGRES_DB=scryfall_cache -e POSTGRES_USER=scryfall \
  -e POSTGRES_PASSWORD=password -p 5432:5432 postgres:16-alpine

export DATABASE_URL="postgresql://scryfall:password@localhost:5432/scryfall_cache"
export API_PORT=8080
cargo run --release
```

**SQLite (standalone):**
```bash
export SQLITE_PATH="./data/scryfall-cache.db"
export PORT=8080
cargo run --release --no-default-features --features sqlite
```

### Other Useful Scripts

```bash
./scripts/generate-openapi.sh        # Generate OpenAPI spec
./scripts/security-scan.sh           # Run security checks
./scripts/benchmark-indexes-v2.sh    # Benchmark query performance
./scripts/test-autocomplete.sh       # Test autocomplete endpoint
./scripts/test-pagination.sh         # Test pagination
```

### Admin Panel (Optional)

```bash
cd admin-panel
npm install
npm run dev        # Dev mode (http://localhost:5173/admin/)
npm run build      # Production build (served at /admin by backend)
```

## Architecture

### Three-Tier Caching Strategy

The core design follows this pattern (see `src/cache/manager.rs:37-122`):

```
User Request → REST API
                  ↓
          CacheManager.search()
                  ↓
    1. Check query_cache table (SHA256 hash)
       ├─ HIT: fetch cards by IDs from cards table
       └─ MISS: ↓
    2. Execute query locally via QueryExecutor
       ├─ SUCCESS: cache result, return cards
       └─ FAIL/EMPTY: ↓
    3. Fall back to Scryfall API (rate-limited)
       └─ Store results in DB + cache
```

**Key insight**: The query parser/executor allows local execution of Scryfall syntax (e.g., `c:red t:creature cmc:<=3`) without hitting the API.

### Dual Database Backend System

Uses **trait abstraction** (`src/db/backend.rs`) to support both databases:

```
DatabaseBackend trait
    ↓
    ├─ PostgresBackend (src/db/postgres/)
    └─ SqliteBackend (src/db/sqlite/)
```

**Feature flags** control which backend is compiled:
- `--features postgres` (default): Production, high concurrency, ~500MB RAM
- `--features sqlite`: Electron/embedded, single file, <100MB RAM

The `Database` struct (`src/db/mod.rs`) wraps the trait and provides a unified API.

### Query System

1. **Parser** (`src/query/parser.rs`): Tokenizes Scryfall syntax into AST
2. **Validator** (`src/query/validator.rs`): Validates parsed queries
3. **Executor** (`src/query/executor.rs`): Converts AST to SQL, executes against local DB
4. **Limits** (`src/query/limits.rs`): Query complexity limits

Supported filters: `name:`, `type:`, `oracle:`, `c:`, `id:`, `set:`, `rarity:`, `cmc:`, `power:`, `toughness:`, `loyalty:`

### Component Structure

```
src/
├── api/              # HTTP endpoints, handlers, middleware, OpenAPI
├── background/       # Bulk data refresh job
├── cache/            # CacheManager (orchestrates 3-tier lookup)
├── circuit_breaker/  # Circuit breaker for Scryfall API failures
├── db/
│   ├── backend.rs    # DatabaseBackend trait
│   ├── postgres/     # PostgreSQL implementation
│   ├── sqlite/       # SQLite implementation
│   └── instrumented.rs  # Metrics wrapper
├── errors/           # Error types and response formatting
├── metrics/          # Prometheus metrics + middleware
├── models/           # Card model (Scryfall schema)
├── query/            # Query parser, validator, executor
├── scryfall/         # API client, rate limiter, bulk loader
└── utils/            # Hashing utilities
```

## Important Patterns

### Database Type Compatibility

**CRITICAL**: Use `DOUBLE PRECISION` in PostgreSQL for fields mapped to Rust `f64`. Using `NUMERIC`/`DECIMAL` will cause "ColumnDecode" errors at runtime (SQLx incompatibility).

Example from `migrations/001_initial_schema.sql:12`:
```sql
cmc DOUBLE PRECISION  -- ✅ Works with f64
```

See MEMORY.md for detailed troubleshooting of this issue.

### Rate Limiting

Uses **GCRA algorithm** (via `governor` crate) in `src/scryfall/rate_limiter.rs`. Configured via `SCRYFALL_RATE_LIMIT_PER_SECOND` (default: 10 req/sec).

### Batch Endpoints for Performance

Three batch endpoints minimize round-trips:
- `POST /cards/batch` - Fetch multiple cards by ID
- `POST /cards/named/batch` - Fetch multiple cards by name
- `POST /queries/batch` - Execute multiple queries in parallel

**Parallelism control**: Set `BATCH_PARALLELISM` (default: 4) to control concurrency.
**Limits**: `BATCH_MAX_IDS`, `BATCH_MAX_NAMES`, `BATCH_MAX_QUERIES`

### Metrics and Observability

- **Prometheus metrics** at `GET /metrics` (see `src/metrics/registry.rs`)
- **Health endpoints**:
  - `/health`, `/health/live` - Liveness (no dependency checks)
  - `/health/ready` - Readiness (checks database, returns 503 if unavailable)
- **Request logging** middleware (`src/api/middleware/logging.rs`)
- **Instance ID** tracking via `INSTANCE_ID` env var for multi-instance deployments

### Background Jobs

Bulk data refresh (`src/background/bulk_refresh.rs`):
- Runs on startup + periodic interval (default: 720 hours = 30 days)
- Downloads ~500MB gzipped JSON from Scryfall's bulk data API
- Inserts cards in batches of 500 for performance
- Set `BULK_REFRESH_ENABLED=false` on secondary instances in multi-instance setups

## Multi-Instance Deployment

The service is **stateless** (state lives in database):
- Multiple instances can run against same database
- Disable bulk refresh on all but one instance: `BULK_REFRESH_ENABLED=false`
- Use `INSTANCE_ID` to identify which instance served a request
- Health endpoints support load balancer health checks

## Environment Variables

See `.env.example` and README.md for full list. Key variables:

**Database:**
- `DATABASE_URL` (PostgreSQL) or `SQLITE_PATH` (SQLite)
- `DATABASE_MAX_CONNECTIONS`, `DATABASE_MIN_CONNECTIONS`

**API Server:**
- `API_HOST`, `API_PORT`
- `INSTANCE_ID` (optional, falls back to `HOSTNAME`)

**Scryfall:**
- `SCRYFALL_RATE_LIMIT_PER_SECOND` (default: 10)
- `BULK_REFRESH_ENABLED`, `BULK_REFRESH_INTERVAL_HOURS`

**Batch Settings:**
- `BATCH_PARALLELISM` (default: 4)
- `BATCH_MAX_IDS`, `BATCH_MAX_NAMES`, `BATCH_MAX_QUERIES`

## Database Schema

Three main tables:

1. **cards**: Full Scryfall card data
   - Primary key: `id` (UUID)
   - 11 indexes for query performance (name, type_line, oracle_text, colors, etc.)
   - Uses `JSONB` for complex fields (prices, legalities)
   - Uses `TEXT[]` for arrays (colors, keywords)

2. **query_cache**: Stores parsed query results
   - Key: `query_hash` (SHA256)
   - Value: `result_ids` (UUID array)
   - TTL tracking via `last_accessed` timestamp

3. **bulk_imports**: Tracks bulk data loads

**Migrations**: Located in `migrations/`, applied automatically on startup (PostgreSQL) or via manual execution (SQLite).

## OpenAPI Documentation

- Generated at build time via `src/bin/openapi_export.rs`
- Served at `GET /api-docs` (Swagger UI)
- Uses `utoipa` crate for automatic schema generation
- Regenerate: `./scripts/generate-openapi.sh`

## Error Handling

Structured errors (`src/errors/`):
- Error codes for client parsing (`src/errors/codes.rs`)
- Consistent JSON response format (`src/errors/response.rs`)
- Always add context to errors with `.context()` or `.map_err()`

## Testing Philosophy

- Unit tests alongside implementation files
- Integration tests use in-memory SQLite for speed
- Benchmark scripts in `scripts/` for performance testing
- CI runs: format check, tests, admin-panel lint/build
