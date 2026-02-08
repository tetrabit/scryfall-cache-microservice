# TypeScript Client

Auto-generated TypeScript client for the Scryfall Cache Microservice API.

## Location

The client lives in `clients/typescript/` within the microservice repository because:

1. **Source of Truth**: The microservice defines the API via OpenAPI spec
2. **Reusability**: Multiple projects can consume this client
3. **Versioning**: Client version tracks API version, not consumer version
4. **Independence**: Can be published or consumed via file reference

## Usage in Projects

### Option 1: File Reference (Recommended for Local Development)

In your project's `package.json`:

```json
{
  "dependencies": {
    "scryfall-cache-client": "file:../scryfall-cache-microservice/clients/typescript"
  }
}
```

Then import:

```typescript
import { ScryfallCacheClient, Card } from 'scryfall-cache-client';

const client = new ScryfallCacheClient({
  baseUrl: 'http://localhost:8080',
});

const response = await client.searchCards({ q: 'lightning bolt' });
```

### Option 2: npm Link (Alternative)

```bash
# In microservice repo
cd clients/typescript
npm link

# In your project
npm link scryfall-cache-client
```

### Option 3: Publish to Registry (Future)

When ready to publish:

```bash
cd clients/typescript
npm publish  # or publish to GitHub Packages
```

## Generation

Generate the client from the root of the microservice repo:

```bash
# Ensure microservice is running
cargo run

# In another terminal
npm run generate:api-types
```

## Architecture

```
scryfall-cache-microservice/
├── clients/
│   └── typescript/          ← TypeScript client lives here
│       ├── package.json     ← Publishable package
│       ├── index.ts         ← Client code
│       ├── schema.d.ts      ← Generated types
│       └── README.md
├── scripts/
│   └── generate-api-types.js  ← Generation script
├── src/                       ← Rust API implementation
└── package.json               ← Scripts to generate client

proxies-at-home/              ← Consumer project
├── package.json              ← References client via file:../
└── server/src/
    └── services/             ← Uses the client
```

This architecture enables:
- ✅ Multiple projects consuming the same client
- ✅ Client versioned with API, not with consumers
- ✅ Easy updates: regenerate client, consumers pick up changes
- ✅ Future-proof: can switch to npm registry later
