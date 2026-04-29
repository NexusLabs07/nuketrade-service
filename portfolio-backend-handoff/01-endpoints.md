# 01 — Endpoints (live)

All three endpoints are mounted under `/aggregated/portfolio/*` and require a JWT bearer token.

```
Authorization: Bearer <jwt>
```

A missing or invalid token returns `401`:
```json
{ "error": "unauthorized", "message": "missing Authorization header" }
```

Validation errors on the path params return `400` with the existing error envelope.

## Mock mode (default while venue plumbing matures)

**These endpoints currently return mock data by default.** No FE change is needed — just call the endpoints normally and you'll see populated UI.

To bypass mock and hit real upstream APIs, pass `?mock=false`:

```
GET /aggregated/portfolio/performance/{evm}/{sol}              # mock (default)
GET /aggregated/portfolio/performance/{evm}/{sol}?mock=false   # real upstream

GET /aggregated/portfolio/pnl-chart/{evm}/{sol}?timeframe=day              # mock
GET /aggregated/portfolio/pnl-chart/{evm}/{sol}?timeframe=day&mock=false   # real

GET /aggregated/portfolio/exchanges/{evm}/{sol}              # mock
GET /aggregated/portfolio/exchanges/{evm}/{sol}?mock=false   # real
```

Path params are validated either way (must be a real-shaped EVM/Solana address). When the real path is mature this default will be flipped back.

---

## 1. Performance stats

```
GET /aggregated/portfolio/performance/{evmAddress}/{solanaAddress}
```

Returns volume + PnL + strategiesOpened for **all 4 timeframes in one call**. The FE switches tabs without refetching.

**Buckets** are rolling windows:
- `day` — last 24 hours
- `week` — last 7 days
- `month` — last 30 days
- `all` — since the user's first activity

**Response (200)**:
```json
{
  "day":   { "volumeUsd": 1234.56,  "strategiesOpened": 3,   "pnlUsd": 12.40 },
  "week":  { "volumeUsd": 9876.10,  "strategiesOpened": 8,   "pnlUsd": -4.20 },
  "month": { "volumeUsd": 50000.00, "strategiesOpened": 22,  "pnlUsd": 312.55 },
  "all":   { "volumeUsd": 250000.0, "strategiesOpened": 102, "pnlUsd": 1820.99 }
}
```

**Empty user**: returns 200 with all zeros.

**Field meanings**:
- `volumeUsd` — sum of `|fillSize × fillPrice|` across Hyperliquid + Pacifica fills inside the bucket.
- `strategiesOpened` — count of `hedge_intents` rows for this user that successfully opened (status in `ACTIVE | CANCELLING | CANCELLED`).
- `pnlUsd` — sum of realized PnL on fills in the bucket. **Does not yet include unrealized PnL deltas** — see [03-caveats.md](03-caveats.md).

---

## 2. PnL chart

```
GET /aggregated/portfolio/pnl-chart/{evmAddress}/{solanaAddress}?timeframe=day|week|month|all
```

Time-series cumulative PnL for the chart. Default `timeframe=day` if omitted.

**Bucket sizes** (resolution per timeframe):
| Timeframe | Bucket | Approx points |
|---|---|---|
| `day`   | 1 hour  | ~24 |
| `week`  | 6 hours | ~28 |
| `month` | 1 day   | ~30 |
| `all`   | 1 week  | ~52 (capped at 365 days back, or first fill) |

**Response (200)**:
```json
{
  "timeframe": "day",
  "rangeStart": "2026-04-24T14:00:00+00:00",
  "rangeEnd":   "2026-04-25T14:00:00+00:00",
  "points": [
    { "timestamp": "2026-04-24T15:00:00+00:00", "cumulativePnlUsd":  4.20 },
    { "timestamp": "2026-04-24T16:00:00+00:00", "cumulativePnlUsd":  3.10 }
  ]
}
```

**Empty user**: 200 with `points: []` and a sensible default range.

**Notes**:
- Timestamps are RFC 3339 / ISO 8601 UTC. Format used: `YYYY-MM-DDTHH:MM:SS+00:00` (offset-suffixed, **not** `Z`-suffixed). JavaScript `new Date(...)` parses both fine.
- `cumulativePnlUsd` starts at 0 at `rangeStart` and accumulates per bucket.
- `tz` query param is accepted but currently ignored — server always buckets in UTC. Re-label client-side if needed.

---

## 3. Exchanges

```
GET /aggregated/portfolio/exchanges/{evmAddress}/{solanaAddress}
```

Per-venue available balance + total equity, plus aggregated totals. Always returns all 4 venue rows in this fixed order: `hyperliquid`, `pacifica`, `lighter`, `backpack`.

**Response (200)**:
```json
{
  "exchanges": [
    {
      "venue": "hyperliquid",
      "displayName": "Hyperliquid",
      "connected": true,
      "availableBalanceUsd": 1230.45,
      "totalEquityUsd": 1450.99,
      "error": null
    },
    {
      "venue": "pacifica",
      "displayName": "Pacifica",
      "connected": true,
      "availableBalanceUsd": 500.00,
      "totalEquityUsd": 520.10,
      "error": null
    },
    {
      "venue": "lighter",
      "displayName": "Lighter",
      "connected": true,
      "availableBalanceUsd": 220.00,
      "totalEquityUsd": 235.50,
      "error": null
    },
    {
      "venue": "backpack",
      "displayName": "Backpack",
      "connected": false,
      "availableBalanceUsd": null,
      "totalEquityUsd": null,
      "error": "not_implemented"
    }
  ],
  "totals": {
    "availableBalanceUsd": 1950.45,
    "totalEquityUsd": 2206.59
  }
}
```

**Field meanings**:
- `connected` — `true` if the user has any balance / equity / position on this venue.
- `availableBalanceUsd` / `totalEquityUsd` — `null` when `connected = false` or upstream errored.
- `error` — short code (e.g. `"upstream_unavailable"`, `"not_implemented"`) or `null` on success. Per-row errors do not break the response.
- `totals` — sum of non-null values across `connected: true` rows.

**Empty user**: all 4 rows present with `connected: false`, balances `null`, totals zeroed (Backpack always has `error: "not_implemented"`).

---

## Error envelope

Same as the rest of the API:
```json
{ "error": "validation_error", "message": "..." }
```

| Status | When |
|---|---|
| 400 | Malformed `evmAddress` / `solanaAddress` / `timeframe` |
| 401 | Missing or invalid JWT |
| 500 | Internal error (rare — upstream venue failures are caught and reported per-row, not as 500) |
