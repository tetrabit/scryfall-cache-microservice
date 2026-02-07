# TODO & Next Steps

This document outlines the immediate tasks that should be completed to move the project forward.

## Immediate Priorities (This Week)

### 🔴 Critical Issues to Fix

#### 1. Fix Bulk Data Loading
**Status**: ⚠️ Currently failing
**Priority**: P0
**Estimated Time**: 2-3 hours

**Problem**: Bulk data download is failing with parse error

**Tasks**:
- [ ] Debug the bulk data API response parsing
- [ ] Add better error messages to identify issue
- [ ] Test with different bulk data types
- [ ] Add retry logic for failed downloads
- [ ] Verify data integrity after import

**Files to modify**:
- `src/scryfall/bulk_loader.rs`

**Testing**:
```bash
docker-compose logs api | grep "bulk"
curl -X POST http://localhost:8080/admin/reload
```

#### 2. Add Basic Authentication
**Status**: 📋 Not started
**Priority**: P0
**Estimated Time**: 4-6 hours

**Why**: Required before public deployment

**Tasks**:
- [ ] Choose auth strategy (API keys recommended)
- [ ] Add API key table to database
- [ ] Create middleware for auth validation
- [ ] Add API key to request headers
- [ ] Add key generation endpoint
- [ ] Document authentication in README

**Implementation approach**:
```rust
// Add to src/api/middleware.rs
pub async fn auth_middleware(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Validate API key
}
```

**New endpoints**:
- `POST /auth/keys` - Generate new API key (admin)
- `DELETE /auth/keys/:id` - Revoke API key (admin)
- `GET /auth/keys` - List API keys (admin)

#### 3. Add Prometheus Metrics
**Status**: 📋 Not started
**Priority**: P0
**Estimated Time**: 3-4 hours

**Why**: Essential for production monitoring

**Tasks**:
- [ ] Add `metrics` crate dependency
- [ ] Create metrics registry
- [ ] Add common metrics (requests, latency, errors)
- [ ] Add cache hit/miss metrics
- [ ] Add database query metrics
- [ ] Create `/metrics` endpoint
- [ ] Add Grafana dashboard JSON

**Metrics to track**:
```
http_requests_total{method, path, status}
http_request_duration_seconds{method, path}
cache_hits_total{tier}
cache_misses_total{tier}
scryfall_api_calls_total
database_queries_total{query_type}
database_query_duration_seconds
```

**Files to create**:
- `src/metrics/mod.rs`
- `grafana/dashboard.json`

## Short Term (This Month)

### 🟡 High Priority Features

#### 4. Improve Error Responses
**Status**: 🔄 Partial
**Priority**: P1
**Estimated Time**: 2-3 hours

**Tasks**:
- [ ] Create structured error types
- [ ] Add error codes to responses
- [ ] Improve error messages for users
- [ ] Add request ID to errors
- [ ] Document error codes

**Example response**:
```json
{
  "success": false,
  "error": {
    "code": "INVALID_QUERY",
    "message": "Query syntax error at position 12",
    "request_id": "req_123456",
    "details": {
      "position": 12,
      "expected": ":",
      "got": "="
    }
  }
}
```

#### 5. Add Query Validation
**Status**: 📋 Not started
**Priority**: P1
**Estimated Time**: 2 hours

**Tasks**:
- [ ] Validate query syntax before execution
- [ ] Return helpful error messages
- [ ] Add query complexity limits
- [ ] Prevent dangerous queries
- [ ] Document query limits

**Files to modify**:
- `src/query/parser.rs`
- `src/api/handlers.rs`

#### 6. Complete Test Coverage
**Status**: 🔄 ~40% coverage
**Priority**: P1
**Estimated Time**: 6-8 hours

**Tasks**:
- [ ] Add integration tests for all endpoints
- [ ] Add database query tests
- [ ] Add cache manager tests
- [ ] Add query executor tests
- [ ] Add bulk loader tests
- [ ] Set up CI to run tests
- [ ] Add test coverage reporting

**Test structure**:
```
tests/
├── integration/
│   ├── api_tests.rs
│   ├── cache_tests.rs
│   └── query_tests.rs
├── unit/
│   ├── parser_tests.rs
│   └── validator_tests.rs
└── fixtures/
    └── sample_cards.json
```

#### 7. Set Up CI/CD
**Status**: 📋 Not started
**Priority**: P1
**Estimated Time**: 2-3 hours

**Tasks**:
- [ ] Create GitHub Actions workflow
- [ ] Add test job
- [ ] Add build job
- [ ] Add Docker build job
- [ ] Add security scanning
- [ ] Add dependency audit
- [ ] Configure branch protection

**Files to create**:
```
.github/
├── workflows/
│   ├── test.yml
│   ├── build.yml
│   ├── security.yml
│   └── release.yml
└── dependabot.yml
```

### 🟢 Nice to Have

#### 8. Add Request Logging
**Status**: 🔄 Basic logging exists
**Priority**: P2
**Estimated Time**: 2 hours

**Tasks**:
- [ ] Add request/response logging middleware
- [ ] Add request ID generation
- [ ] Log query parameters
- [ ] Log response times
- [ ] Add correlation IDs

#### 9. Improve Documentation
**Status**: 🔄 Basic docs exist
**Priority**: P2
**Estimated Time**: 3-4 hours

**Tasks**:
- [ ] Add architecture diagram
- [ ] Add sequence diagrams
- [ ] Create API documentation site
- [ ] Add more code examples
- [ ] Create video tutorial
- [ ] Add troubleshooting guide

#### 10. Add Admin Panel
**Status**: 📋 Not started
**Priority**: P2
**Estimated Time**: 8-10 hours

**Tasks**:
- [ ] Create simple web UI
- [ ] Add cache statistics dashboard
- [ ] Add query logs viewer
- [ ] Add API key management
- [ ] Add system health monitor
- [ ] Add configuration editor

## Medium Term (Next 3 Months)

### Phase 1: Production Hardening
- [ ] Add circuit breaker for Scryfall API
- [ ] Add request timeout configuration
- [ ] Add graceful shutdown
- [ ] Add database migration versioning
- [ ] Add configuration validation
- [ ] Add health check improvements
- [ ] Add backup/restore procedures

### Phase 2: Performance Optimization
- [ ] Add Redis cache layer
- [ ] Optimize database queries
- [ ] Add response compression
- [ ] Add connection pooling tuning
- [ ] Add query result caching
- [ ] Profile and optimize hot paths

### Phase 3: Feature Expansion
- [ ] Add GraphQL API
- [ ] Add WebSocket support
- [ ] Add batch operations
- [ ] Add image caching
- [ ] Add price tracking
- [ ] Add collection management

## Long Term (6+ Months)

### Phase 4: Ecosystem
- [ ] Build client SDKs (TS, Python, Go)
- [ ] Create web frontend
- [ ] Create mobile apps
- [ ] Add third-party integrations
- [ ] Build community

### Phase 5: Scale
- [ ] Multi-region deployment
- [ ] Horizontal scaling
- [ ] Read replicas
- [ ] CDN integration
- [ ] Edge computing

## Quick Fixes (< 30 min each)

- [ ] Remove unused imports
- [ ] Fix compiler warnings
- [ ] Add version to health endpoint
- [ ] Add environment to health endpoint
- [ ] Improve log messages
- [ ] Add more examples to README
- [ ] Fix typos in documentation
- [ ] Add license file
- [ ] Add contributing guide
- [ ] Add code of conduct

## Code Quality Tasks

- [ ] Run `cargo clippy` and fix warnings
- [ ] Run `cargo fmt` on all files
- [ ] Remove dead code
- [ ] Add rustdoc comments to public APIs
- [ ] Add module-level documentation
- [ ] Improve variable naming
- [ ] Extract magic numbers to constants
- [ ] Add type aliases for complex types

## Documentation Tasks

- [ ] Complete API reference
- [ ] Add OpenAPI/Swagger spec
- [ ] Create Postman collection
- [ ] Add curl examples for all endpoints
- [ ] Document error codes
- [ ] Add deployment guide
- [ ] Add scaling guide
- [ ] Add security best practices

## Testing Strategy

### Current State
- ✅ Basic unit tests for parser
- ✅ Basic unit tests for rate limiter
- ⚠️ No integration tests
- ❌ No load tests
- ❌ No e2e tests

### Goal State
- ✅ 80%+ unit test coverage
- ✅ Integration tests for all endpoints
- ✅ Load tests for performance validation
- ✅ E2E tests for critical flows
- ✅ Chaos engineering tests

### Test Checklist
- [ ] Test all success paths
- [ ] Test all error paths
- [ ] Test edge cases
- [ ] Test concurrency
- [ ] Test rate limiting
- [ ] Test caching behavior
- [ ] Test database failures
- [ ] Test Scryfall API failures
- [ ] Test query parsing edge cases
- [ ] Test authentication (when added)

## Performance Goals

### Current Performance
- Cache hit: ~10ms
- Database query: ~50ms
- API fallback: ~300ms

### Target Performance
- Cache hit: <5ms
- Database query: <20ms
- API fallback: <200ms
- p99 latency: <100ms
- Throughput: 10,000 req/min

### Optimization Checklist
- [ ] Profile application
- [ ] Identify bottlenecks
- [ ] Optimize hot paths
- [ ] Add database indexes
- [ ] Tune connection pool
- [ ] Add Redis cache
- [ ] Enable compression
- [ ] Optimize JSON parsing
- [ ] Batch database operations
- [ ] Use prepared statements

## Security Checklist

- [ ] Add authentication
- [ ] Add authorization
- [ ] Add rate limiting per user
- [ ] Add input validation
- [ ] Add SQL injection prevention
- [ ] Add XSS prevention
- [ ] Add CSRF protection
- [ ] Add secure headers
- [ ] Add TLS support
- [ ] Add secrets management
- [ ] Scan dependencies for vulnerabilities
- [ ] Add security headers
- [ ] Add CORS configuration
- [ ] Add request size limits
- [ ] Sanitize log output

## Deployment Checklist

### Pre-deployment
- [ ] Run all tests
- [ ] Update dependencies
- [ ] Security audit
- [ ] Performance benchmarks
- [ ] Load testing
- [ ] Documentation review
- [ ] Backup database
- [ ] Create rollback plan

### Deployment
- [ ] Deploy to staging
- [ ] Smoke tests
- [ ] Deploy to production
- [ ] Monitor metrics
- [ ] Check error rates
- [ ] Verify functionality
- [ ] Update status page

### Post-deployment
- [ ] Monitor for 24 hours
- [ ] Review logs
- [ ] Check performance
- [ ] Gather feedback
- [ ] Document issues
- [ ] Plan improvements

## Knowledge Gaps to Address

- [ ] Learn more about Scryfall query syntax edge cases
- [ ] Research best practices for async Rust
- [ ] Study PostgreSQL query optimization
- [ ] Learn about distributed caching strategies
- [ ] Understand rate limiting algorithms
- [ ] Study API authentication patterns
- [ ] Learn Prometheus and Grafana
- [ ] Study Docker optimization
- [ ] Learn Kubernetes (for future scaling)

## Resources Needed

### Tools
- [ ] Grafana for monitoring
- [ ] Prometheus for metrics
- [ ] Redis for caching (future)
- [ ] Load testing tool (k6, wrk)
- [ ] APM tool (optional)

### Services
- [ ] CI/CD platform (GitHub Actions)
- [ ] Container registry (Docker Hub, GHCR)
- [ ] Hosting (AWS, GCP, DigitalOcean)
- [ ] Domain name
- [ ] SSL certificate
- [ ] CDN (optional)

### Team
- [ ] Code reviewer
- [ ] Security reviewer
- [ ] Performance expert (optional)
- [ ] Technical writer (optional)

## Questions to Answer

1. What authentication method to use?
   - API keys (recommended)
   - OAuth 2.0
   - JWT tokens

2. Should we support multiple bulk data types?
   - default_cards (currently)
   - oracle_cards
   - all_cards
   - rulings

3. What's the cache eviction strategy?
   - LRU (Least Recently Used)
   - LFU (Least Frequently Used)
   - TTL (Time To Live)

4. Should we support WebSockets?
   - For real-time updates
   - For price changes
   - For new card releases

5. What testing framework to standardize on?
   - Built-in Rust testing
   - Criterion for benchmarks
   - Testcontainers for integration tests

## Decision Log

### Decisions Made
1. ✅ Use Rust for high performance
2. ✅ Use PostgreSQL for robust storage
3. ✅ Use Axum for web framework
4. ✅ Use Docker for deployment
5. ✅ Use GCRA for rate limiting

### Decisions Pending
1. ⏳ Authentication method
2. ⏳ Monitoring solution
3. ⏳ Caching strategy (Redis vs in-memory)
4. ⏳ WebSocket support
5. ⏳ GraphQL vs REST (or both)

## Getting Started with Contributions

New contributors should start with:
1. Read DEVELOPMENT.md
2. Set up local environment
3. Pick a "good first issue"
4. Write tests
5. Submit PR

Good first issues:
- Add version to health endpoint
- Improve error messages
- Add more examples to README
- Fix compiler warnings
- Add rustdoc comments
