# TD Task Management - Scryfall Cache Microservice

This project uses **td** (marcus/td) for AI-assisted task management with structured handoffs, session isolation, and dependency tracking.

## Quick Start

### Starting a New Session
```bash
# Always run this at the start of a conversation
td usage --new-session

# Or just check current state
td usage -q
```

### What to Work On
```bash
# Show the next highest priority task
td next

# Show optimal work sequence
td critical-path

# Show all tasks by priority
td query "status = open" --sort priority
```

## Task Structure

### Summary
- **Total Tasks**: 36 open tasks
- **P0 Critical**: 3 tasks (must do before production)
- **P1 High Priority**: 4 tasks (this month)
- **P2 Medium Priority**: 7 tasks (nice to have)
- **P3 Future**: 10 tasks (long term)
- **Quick Fixes**: 8 minor self-reviewable tasks
- **Epics**: 5 major initiatives

### ✅ All Tasks Have Detailed Descriptions

Every task includes:
- Why it's needed and current state
- Detailed implementation approach with code examples
- Files to create/modify
- Configuration examples
- Dependencies and tools needed
- Testing commands
- Clear acceptance criteria
- Related task dependencies

View any task's full description with: `td context <task-id>`

## Critical Path

The optimal work sequence to unblock the most tasks:

1. **td-75f131** - Implement structured error response system (unblocks 5 tasks)
2. **td-6ecdde** - Add Prometheus metrics endpoint (unblocks 3 tasks)
3. **td-811f8f** - Set up read replicas for database (unblocks 2 tasks)

These are the **bottlenecks** - resolving them will enable the most parallel work.

## P0 - Critical Tasks (Before Production)

1. **td-ca6ded** - Fix bulk data loading errors (bug)
2. **td-208002** - Add API key authentication system (feature)
3. **td-6ecdde** - Add Prometheus metrics endpoint (feature)

Start with: `td start td-ca6ded`

## P1 - High Priority (This Month)

1. **td-75f131** - Implement structured error response system (unblocks 5 tasks!)
2. **td-a5d2d4** - Add query syntax validation and limits
3. **td-504f6d** - Achieve 80%+ test coverage with integration tests
4. **td-be7609** - Set up GitHub Actions CI/CD pipeline (depends on tests)
5. **td-193505** - Add security scanning and dependency audit

## P2 - Medium Priority (Nice to Have)

### Production Hardening
- **td-48c39f** - Add structured request/response logging middleware
- **td-f3d6f2** - Add circuit breaker for Scryfall API failures
- **td-8bce67** - Implement graceful shutdown handling

### Developer Experience
- **td-beb90c** - Build web-based admin panel for monitoring
- Quick fix tasks (compiler warnings, formatting, docs)

### Performance
- **td-4e9215** - Optimize database queries and add prepared statements
- **td-6d2773** - Add gzip/brotli response compression

## P3 - Future Features (Long Term)

### Feature Expansion
- **td-1f72c9** - Add Redis cache layer for query results
- **td-ae0414** - Add GraphQL API endpoint
- **td-015f8a** - Add WebSocket support for real-time updates
- **td-949187** - Add batch operation endpoints
- **td-2684cb** - Add image caching for card images
- **td-ddb3ea** - Add price tracking and history
- **td-5aac0b** - Build client SDKs (TypeScript, Python, Go)

### Scale & Infrastructure
- **td-811f8f** - Set up read replicas for database
- **td-d8d894** - Add horizontal scaling support
- **td-1b5e51** - Implement multi-region deployment

## Epics (Major Initiatives)

1. **td-7142e3** - Production Hardening
2. **td-9ed227** - Performance Optimization
3. **td-dfd453** - Feature Expansion
4. **td-db8687** - Developer Experience
5. **td-abd94e** - Scale & Infrastructure

View epic hierarchy: `td tree <epic-id>`

## Available Boards

```bash
# Critical path - highest impact work
td board show critical-path

# Sprint planning - all P0-P1 work
td board show sprint-1

# Production readiness tracking
td board show production-readiness

# Feature development
td board show feature-development

# Quick wins - easy tasks
td board show quick-wins
```

## Workflow

### Starting Work
```bash
# Start working on a task
td start td-ca6ded

# Link files you'll modify
td link td-ca6ded src/scryfall/bulk_loader.rs

# For multiple related tasks, use workspace
td ws start "Auth implementation"
td ws tag td-208002 td-193505
```

### During Work
```bash
# Log progress regularly
td log "fixed parsing error in bulk data handler"
td log "added retry logic with exponential backoff"

# Log important decisions
td log --decision "using serde_json for flexible parsing to handle both gzipped and plain JSON"

# Log blockers
td log --blocker "need clarification on auth token format"
```

### Completing Work
```bash
# Handoff with structured context
td handoff td-ca6ded \
  --done "fixed JSON parsing, added retry logic, tested with production data" \
  --remaining "need to add monitoring metrics, update documentation"

# Submit for review (different session must approve)
td review td-ca6ded
```

### Review (Different Session)
```bash
# Check what needs review
td reviewable

# Approve or reject
td approve td-ca6ded
# or
td reject td-ca6ded --reason "missing error handling for network failures"
```

## Task Dependencies

Key dependency chains:

1. **Error Handling Chain**:
   - td-75f131 (error responses) → td-504f6d (tests) → td-be7609 (CI/CD)
   - Also unblocks: GraphQL, WebSocket, SDKs

2. **Metrics Chain**:
   - td-6ecdde (metrics) → td-beb90c (admin panel)
   - Also unblocks: td-d8d894 (scaling)

3. **Infrastructure Chain**:
   - td-811f8f (read replicas) → td-d8d894 (scaling) → td-1b5e51 (multi-region)

4. **API Stability Chain**:
   - td-75f131 (errors) + td-a5d2d4 (validation) → td-5aac0b (SDKs)

## Common Commands

```bash
# Session management
td usage --new-session      # Start new session
td current                  # What you're working on
td ws current               # Current workspace state

# Finding work
td next                     # Highest priority task
td critical-path            # Optimal work sequence
td query "priority <= P1"   # Custom queries

# Working
td start <id>               # Begin work
td log "message"            # Log progress
td log --decision "..."     # Log decision
td log --blocker "..."      # Log blocker
td link <id> <files>        # Link files

# Handoffs
td handoff <id> --done "..." --remaining "..."
td ws handoff               # Handoff entire workspace

# Review
td review <id>              # Submit for review
td reviewable               # What you can review
td approve <id>             # Approve work
td reject <id> --reason "..." # Reject with feedback

# Organization
td board show <name>        # View board
td dep <id>                 # View dependencies
td tree <epic-id>           # View epic hierarchy
td monitor                  # Live dashboard
```

## Recommended Work Sequence

### Week 1: Production Hardening (P0)
1. Fix bulk data loading (td-ca6ded)
2. Add authentication (td-208002)
3. Add metrics (td-6ecdde)

### Week 2-3: API Stability (P1)
1. Implement error handling (td-75f131) - **CRITICAL: Unblocks 5 tasks**
2. Add query validation (td-a5d2d4)
3. Write comprehensive tests (td-504f6d)
4. Set up CI/CD (td-be7609)
5. Add security scanning (td-193505)

### Week 4: Polish & Documentation
1. Add request logging (td-48c39f)
2. Build admin panel (td-beb90c)
3. Improve documentation
4. Quick fixes and cleanup

### Month 2+: Features & Scale
- Redis caching (td-1f72c9)
- GraphQL API (td-ae0414)
- WebSocket support (td-015f8a)
- Database optimization (td-4e9215)
- Infrastructure scaling (td-811f8f, td-d8d894, td-1b5e51)

## Best Practices

1. **Always start sessions properly**: `td usage --new-session`
2. **Log liberally**: Capture decisions, progress, blockers as you work
3. **Link files early**: `td link <id> <files>` when starting implementation
4. **Structured handoffs**: Always use `--done` and `--remaining`
5. **Respect session isolation**: Never approve work you implemented
6. **Check dependencies**: Use `td dep <id>` before starting work
7. **Follow critical path**: Resolve bottlenecks first to unblock others
8. **Use workspaces**: For multi-issue work, use `td ws` commands

## Analytics & Insights

```bash
# Find stale work
td query "stale(7)"

# Find rejected work needing rework
td query "rework()"

# Count tasks by status
td query "status = in_progress" --output count

# Find unblocked work
td query "status = open" --sort priority

# See who's working on what
td query "status = in_progress"
```

## Quick Reference Card

| Command | Purpose |
|---------|---------|
| `td next` | Show next task |
| `td critical-path` | Show bottlenecks |
| `td start <id>` | Begin work |
| `td log "msg"` | Log progress |
| `td handoff <id>` | Pause work |
| `td review <id>` | Submit for review |
| `td approve <id>` | Complete review |
| `td board show <name>` | View board |
| `td monitor` | Live dashboard |

## Files Modified

This task structure was created from:
- `TODO.md` - Comprehensive project TODO list
- `CHANGELOG.md` - Project changelog and roadmap
- `README.md` - Project overview and features

All tasks are tracked in `.todos/` directory (SQLite database + session state).

## Support

For td help:
```bash
td --help
td <command> --help
td query --examples
```

For project-specific questions, see:
- `README.md` - Project documentation
- `DEVELOPMENT.md` - Development guide
- `QUICKSTART.md` - Quick start guide

---

**Remember**: The session that implements cannot approve. This enforces code review and prevents "works on my context" bugs. Start each new conversation with `td usage --new-session`!
