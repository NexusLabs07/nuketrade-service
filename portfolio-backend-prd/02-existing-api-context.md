# 02 — Existing API Context

The new portfolio endpoints must align with conventions already in production. Everything you need to know is inlined here.

## Base URL & infra

- **Base URL** is configured per-environment via `NEXT_PUBLIC_API_URL` on the FE. Typical local default: `http://localhost:8000`. Production: a hosted domain. The exact prod base URL is owned by infra.
- **CORS**: configured server-side via `CORS_ALLOWED_ORIGINS`. Defaults already include `http://localhost:3000` and prod domains.

## Top-level route prefixes already in use

| Prefix | Purpose |
|---|---|
| `/` | Health check (`GET /`) |
| `/auth` | Login / JWT issuance |
| `/user` | User profile, claim flows |
| `/hyperliquid` | Hyperliquid metadata, positions, deposit helpers |
| `/pacifica` | Pacifica metadata, positions, deposit helpers |
| `/lighter` | Lighter perp metadata |
| `/aggregated` | Cross-venue endpoints (positions, charts, live feed, APR) |
| `/bridge` | Solana-origin relay quotes + permit execution |
| `/hedge-intents` | Hedge intent state machine |
| `/withdraw-intents` | Withdrawal intent state machine (auth required) |

**The new portfolio endpoints belong under `/aggregated/portfolio/*`** since they aggregate across venues, matching the existing `/aggregated/*` family.

## Auth model

- JWT-based. The FE attaches `Authorization: Bearer <jwt>` to:
  - All non-GET requests
  - GET requests on `/withdraw-intents/*`
- GETs on other prefixes are **currently unauthenticated and identify the user via path params** (EVM + Solana addresses). Examples already in production:

```
GET /aggregated/open-positions/{evmAddress}/{solanaAddress}
```

**The new portfolio endpoints follow the same pattern** — public GET, user identified by `{evmAddress}/{solanaAddress}` path params. If the backend team wants to require JWT on these new endpoints instead, that's fine; flag it so we can update the FE client.

## Request/response conventions

- **Content type**: `application/json` for both directions.
- **HTTP methods**: only `GET` is needed for the portfolio page (read-only).
- **Path-param identity**: `{evmAddress}` is a 0x-prefixed lowercase hex string (42 chars). `{solanaAddress}` is a base58 string. Validate format, return `400` on malformed addresses.
- **Empty user state**: when a user exists but has no data, **return `200` with zeroed/empty values, not `404`**. The FE renders the empty state directly.
- **Error envelope**: existing endpoints return errors as:
  ```json
  { "message": "human-readable message" }
  ```
  with a non-2xx HTTP status. Use this same shape.

## Venue keys (case-sensitive, lowercase)

The product uses these lowercase identifiers consistently across DB, charts, and FE:

```
hyperliquid
pacifica
backpack
lighter
```

The portfolio page UI shows "trade[xyz]" as a fifth venue. Whether this becomes a real venue with its own key, or stays a placeholder, is an open question (see [05-open-questions.md](05-open-questions.md)). For now, return the four venues above; if "trade[xyz]" becomes real, add a key for it later.

## Existing aggregated endpoints (for reference)

These are already live and the new endpoints should match their style:

### `GET /aggregated/open-positions/{evmAddress}/{solanaAddress}`
Returns an array of cross-venue position objects, with per-venue legs nested:

```json
[
  {
    "symbol": "BTC",
    "hyperliquid": {
      "symbol": "BTC", "size": "0.05", "side": "Long",
      "pnl": "12.40", "funding": "0.50", "margin": "1000.00",
      "leverage": 10, "liquidationPrice": "85000.00"
    },
    "pacifica": { /* same shape */ },
    "backpack": { /* same shape, optional */ },
    "lighter":  { /* same shape, optional */ }
  }
]
```

Note: the legacy positions endpoint returns numeric values **as strings**. **For the new portfolio endpoints, please return numerics as JSON numbers** (see [03-endpoints.md](03-endpoints.md)) — the FE handles formatting and aggregation.

### `GET /aggregated/live/market-feed`
Returns per-symbol mark price / funding / max leverage across venues. Demonstrates the venue-key convention:

```json
{
  "symbol": "BTC",
  "hyperliquid": { "mark_px": 95000.0, "funding": 0.0001, "max_leverage": 50 },
  "pacifica":    { "mark_px": 94990.0, "funding": 0.00012, "max_leverage": 20 },
  "backpack":    null,
  "lighter":     { "mark_px": 95005.0, "funding": 0.000011, "max_leverage": 50 }
}
```

A whole-venue `null` means "no data for this venue yet."

### `GET /aggregated/chart/{assetName}?timeframe=30m|1h|24h`
Demonstrates query-param timeframes and a per-venue keyed time-series response:

```json
{
  "hyperliquid": [
    { "id": "...", "platform": "hyperliquid", "symbol": "BTC",
      "rate": 0.0001, "mark_px": 95000.0, "timestamp": "2026-04-25T10:00:00Z" }
  ],
  "pacifica":  [ /* ... */ ],
  "backpack":  [ /* ... */ ],
  "lighter":   [ /* ... */ ]
}
```

Use the same `timestamp` (ISO 8601 UTC) format and `timeframe` query-param style on the new portfolio endpoints.

## Caching / freshness expectations

The FE typically polls aggregated endpoints every 10–30 seconds. Server-side caching of 10–15s per user for the new portfolio endpoints is fine. For long-window chart data (e.g. monthly/all-time) a longer cache (5–10 min) is acceptable.

There is no websocket/SSE channel for portfolio data today. If one is added later, polling can be replaced — out of scope for v1.
