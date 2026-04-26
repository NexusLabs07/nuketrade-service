# 03 — Caveats & Known Limits

Things the FE should know that aren't obvious from the schemas.

## Backpack is stubbed

The Backpack row in the `exchanges` response is always:
```json
{
  "venue": "backpack",
  "displayName": "Backpack",
  "connected": false,
  "availableBalanceUsd": null,
  "totalEquityUsd": null,
  "error": "not_implemented"
}
```

**Why:** Backpack's account API requires per-user ED25519 API key signing. The product doesn't store user-keyed Backpack credentials yet. Until that's wired up, Backpack will render as `--` like any disconnected venue.

**FE action:** Treat `error === "not_implemented"` the same as `connected === false` — show `--`. No special UI needed.

## Lighter has no historical buckets

Lighter's user-trades endpoint isn't documented for the params we tried. As a result:

- **Exchanges endpoint** — Lighter balance + equity work fine (we use `accountsByL1Address`).
- **Performance / PnL chart** — Lighter fills are **not** included in the volume / PnL aggregation. Hyperliquid + Pacifica fills are.

So if a user only trades on Lighter, their Performance numbers will read 0 even though Lighter shows balance/equity. Once the Lighter trades endpoint is figured out, this will be backfilled — no schema change.

## "trade[xyz]" was dropped

Per the open-questions resolution, this placeholder card is not in the API. The FE should remove its hardcoded card.

## PnL definition (current implementation)

`pnlUsd` in both the performance buckets and the chart points is **realized PnL on closed fills only** — sum of `closedPnl` from Hyperliquid `userFills` plus `pnl` from Pacifica `trades/history` inside the window.

It does **not** yet include:
- Unrealized PnL (current open position mark-to-market)
- Funding payments

The PRD asked for realized + unrealized + funding; we shipped realized only as v1 because computing the unrealized delta inside a rolling window requires equity snapshots we don't take yet. Will be revisited.

**FE action:** No change — just be aware the number may understate "true" PnL until v2.

## `tz` query param is accepted but ignored

The PnL chart endpoint accepts `?tz=America/New_York` but currently buckets in UTC regardless. The FE can re-label the chart axis client-side using the timestamp values.

## Auth is JWT, not public

This differs from sibling endpoints like `/aggregated/open-positions/...` which are public. Portfolio endpoints **require** `Authorization: Bearer <jwt>` — the FE's existing API client already attaches the token for `/withdraw-intents`; copy that pattern.

## Polling cadence

Same recommendation as the PRD: poll `performance` and `exchanges` every 10–15s, `pnl-chart?timeframe=day` every 10–15s, longer timeframes can poll less often. There's no WebSocket channel for portfolio data yet.
