# 03 — New Endpoints

Three new endpoints under `/aggregated/portfolio/*`. All are read-only GETs, identifying the user via path params, matching existing aggregated endpoints (see [02-existing-api-context.md](02-existing-api-context.md)).

---

## 3.1 Performance stats

### `GET /aggregated/portfolio/performance/{evmAddress}/{solanaAddress}`

Returns aggregate performance metrics for **all four timeframes in a single call**, so the FE can switch tabs (Day / Week / Month / All) without refetching.

**Path params**
| Name | Type | Notes |
|---|---|---|
| `evmAddress` | string | 0x-prefixed lowercase hex, 42 chars |
| `solanaAddress` | string | base58 |

**Query params**
| Name | Type | Default | Notes |
|---|---|---|---|
| `tz` | string (IANA timezone) | `UTC` | Used to compute "Day" / "Week" / "Month" boundaries when calendar-aligned (e.g. `America/New_York`). Optional. |

**Response — `200 OK`**
```json
{
  "day":   { "volumeUsd": 1234.56,  "strategiesOpened": 3,   "pnlUsd":   12.40 },
  "week":  { "volumeUsd": 9876.10,  "strategiesOpened": 8,   "pnlUsd":   -4.20 },
  "month": { "volumeUsd": 50000.00, "strategiesOpened": 22,  "pnlUsd":  312.55 },
  "all":   { "volumeUsd": 250000.0, "strategiesOpened": 102, "pnlUsd": 1820.99 }
}
```

**Field definitions**
- `volumeUsd` — total notional traded by this user across all venues during the bucket.
- `strategiesOpened` — number of distinct hedge/arb strategies opened during the bucket. (Decision needed: hedge intents created vs. positions opened — see [05-open-questions.md](05-open-questions.md).)
- `pnlUsd` — realized + unrealized + funding PnL during the bucket. (Confirm definition in [05-open-questions.md](05-open-questions.md).)
- All numbers are JSON numbers (not strings). Negative values are allowed for `pnlUsd`.

**Bucket semantics** (recommended; backend may override and document)
- `day` — rolling last 24 hours.
- `week` — rolling last 7 days.
- `month` — rolling last 30 days.
- `all` — since the user's first trade.

Rolling windows are recommended over calendar-aligned because they're consistent with the chart endpoint and avoid timezone edge cases. If calendar alignment is preferred, use `tz`.

**Empty / no-activity user**
Return `200` with all values set to `0`:
```json
{
  "day":   { "volumeUsd": 0, "strategiesOpened": 0, "pnlUsd": 0 },
  "week":  { "volumeUsd": 0, "strategiesOpened": 0, "pnlUsd": 0 },
  "month": { "volumeUsd": 0, "strategiesOpened": 0, "pnlUsd": 0 },
  "all":   { "volumeUsd": 0, "strategiesOpened": 0, "pnlUsd": 0 }
}
```

---

## 3.2 PnL chart time series

### `GET /aggregated/portfolio/pnl-chart/{evmAddress}/{solanaAddress}`

Returns time-series PnL data points for the chart on the Performance section.

**Path params** — same as 3.1.

**Query params**
| Name | Type | Required | Notes |
|---|---|---|---|
| `timeframe` | `day` \| `week` \| `month` \| `all` | yes | Matches the active tab. |
| `tz` | string (IANA) | no | Default `UTC`. |

**Response — `200 OK`**
```json
{
  "timeframe": "day",
  "rangeStart": "2026-04-24T14:00:00Z",
  "rangeEnd":   "2026-04-25T14:00:00Z",
  "points": [
    { "timestamp": "2026-04-24T14:00:00Z", "cumulativePnlUsd":  0.00 },
    { "timestamp": "2026-04-24T15:00:00Z", "cumulativePnlUsd":  4.20 },
    { "timestamp": "2026-04-24T16:00:00Z", "cumulativePnlUsd": -1.10 }
  ]
}
```

**Field definitions**
- `timeframe` — echoed from the query param.
- `rangeStart` / `rangeEnd` — ISO 8601 UTC, the inclusive window for which points are returned.
- `points[].timestamp` — ISO 8601 UTC, the bucket-end time.
- `points[].cumulativePnlUsd` — cumulative PnL within this window. **Starts at `0` at `rangeStart`** and accumulates. PnL is realized + unrealized + funding (see [05-open-questions.md](05-open-questions.md)).

**Recommended resolution**
| Timeframe | Bucket size | Approx point count |
|---|---|---|
| `day` | 1 hour | ~24 |
| `week` | 6 hours | ~28 |
| `month` | 1 day | ~30 |
| `all` | 1 week | capped at ~52 (downsample if longer) |

Backend may choose different resolutions; document them.

**Empty / no-activity user**
Return `200` with `points: []` and a sensible default `rangeStart`/`rangeEnd` (e.g. last 24h for `day`).

---

## 3.3 Exchanges (per-venue balances)

### `GET /aggregated/portfolio/exchanges/{evmAddress}/{solanaAddress}`

Returns per-exchange available balance and total equity, plus an aggregated total.

**Path params** — same as 3.1.

**Query params** — none.

**Response — `200 OK`**
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
      "venue": "backpack",
      "displayName": "Backpack",
      "connected": true,
      "availableBalanceUsd": 500.00,
      "totalEquityUsd": 500.00,
      "error": null
    },
    {
      "venue": "pacifica",
      "displayName": "Pacifica",
      "connected": false,
      "availableBalanceUsd": null,
      "totalEquityUsd": null,
      "error": null
    },
    {
      "venue": "lighter",
      "displayName": "Lighter",
      "connected": true,
      "availableBalanceUsd": 220.00,
      "totalEquityUsd": 235.50,
      "error": null
    }
  ],
  "totals": {
    "availableBalanceUsd": 1950.45,
    "totalEquityUsd": 2186.49
  }
}
```

**Field definitions**
- `venue` — stable lowercase id from the venue-key convention (`hyperliquid` | `pacifica` | `backpack` | `lighter`, plus any future additions like `tradexyz` if "trade[xyz]" becomes real).
- `displayName` — user-facing label. Returning this server-side avoids hardcoding labels on the FE and keeps display naming centralized.
- `connected` — `true` if this user has set up / deposited on this venue. `false` otherwise.
- `availableBalanceUsd` — withdrawable / free margin in USD. `null` when `connected = false` or when the upstream venue API is failing (see `error`).
- `totalEquityUsd` — total account value (collateral + unrealized PnL) in USD. `null` under the same conditions.
- `error` — `null` on success, otherwise a short string code describing why the values are unavailable for this row (e.g. `"upstream_unavailable"`). Per-row errors must not break the response — other rows and `totals` should still render.
- `totals` — sum of `availableBalanceUsd` and `totalEquityUsd` over rows where `connected = true` and values are not `null`.

**Order of rows**
The FE expects this order (matching the page layout): `hyperliquid`, `backpack`, `pacifica`, `lighter`. If "trade[xyz]" is added later, slot it after `lighter`. The "All Exchanges" aggregate is rendered from `totals`, not as a row.

**Empty / no-activity user**
Return `200`. All four rows present with `connected: false`, both balance fields `null`, and `totals` zeroed:
```json
{
  "exchanges": [
    { "venue": "hyperliquid", "displayName": "Hyperliquid", "connected": false, "availableBalanceUsd": null, "totalEquityUsd": null, "error": null },
    { "venue": "backpack",    "displayName": "Backpack",    "connected": false, "availableBalanceUsd": null, "totalEquityUsd": null, "error": null },
    { "venue": "pacifica",    "displayName": "Pacifica",    "connected": false, "availableBalanceUsd": null, "totalEquityUsd": null, "error": null },
    { "venue": "lighter",     "displayName": "Lighter",     "connected": false, "availableBalanceUsd": null, "totalEquityUsd": null, "error": null }
  ],
  "totals": { "availableBalanceUsd": 0, "totalEquityUsd": 0 }
}
```

---

## Error responses (all three endpoints)

Use the existing error envelope:

```json
{ "message": "Invalid EVM address" }
```

| HTTP status | When |
|---|---|
| `400` | Malformed `evmAddress` / `solanaAddress` / `timeframe`. |
| `401` | (Only if the team chooses to gate these behind JWT — currently not required.) |
| `500` | Unhandled internal failure. Upstream venue failures should NOT bubble up as 500 — they should be reported in the per-row `error` field on the exchanges endpoint, leaving the rest of the response intact. |

---

## Caching guidance

| Endpoint | Recommended server-side cache |
|---|---|
| `performance` | 10–15 s per user |
| `pnl-chart?timeframe=day` | 10–15 s per user |
| `pnl-chart?timeframe=week` | 30–60 s per user |
| `pnl-chart?timeframe=month` | 5 min per user |
| `pnl-chart?timeframe=all` | 10 min per user |
| `exchanges` | 10–15 s per user |
