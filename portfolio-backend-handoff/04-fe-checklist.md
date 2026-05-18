# 04 — FE Checklist

What the FE engineer needs to do to wire up the portfolio page.

## 1. Add API client functions

Create three functions that hit the new endpoints. They MUST attach the JWT bearer token (use the same helper as `/withdraw-intents` calls).

```ts
// All three: GET, JSON response, requires Bearer JWT.

getPortfolioPerformance(evm: string, sol: string)
  -> Promise<PerformanceResponse>

getPortfolioPnlChart(evm: string, sol: string, timeframe: PerformanceTimeframe)
  -> Promise<PnlChartResponse>

getPortfolioExchanges(evm: string, sol: string)
  -> Promise<ExchangesResponse>
```

Base URL: `${NEXT_PUBLIC_API_URL}/aggregated/portfolio/...`

## 2. Drop in the types

Copy [02-types.md](02-types.md) into the FE's API types file. They are the canonical contract.

## 3. Replace placeholder data on `/portfolio`

- **Performance section** — `usePerformanceQuery()` hook, fetches once on mount. Switching the Day/Week/Month/All tab just selects which key to read from the response — no refetch.
- **PnL chart** — fetch separately per timeframe. Refetch when the user switches tabs.
- **Exchanges grid** — render the 4 rows from `exchanges[]` in the order returned. Render `--` whenever `connected === false` OR `availableBalanceUsd === null`. The "All Exchanges" aggregate card reads from `totals`.

## 4. Drop the `trade[xyz]` card

Remove it from the FE — the API does not return it.

## 5. Format numbers client-side

The API returns raw numbers. The FE formats them:
- USD with 2 decimals (`$1,234.56`) — except small values where you may want more precision.
- PnL: prefix with `+` or `-`, color green/red.
- Negative volume is impossible; treat it as a bug.

## 6. Empty states

- All-zeros performance / empty `points: []` chart → render the same empty UI you use today.
- Backpack will always show `--` in v1 — that's expected.

## 7. Polling

Match existing aggregated endpoints — `setInterval` 10–15 s for performance + exchanges + day chart. Longer timeframes can poll less often if needed.

## 8. Error handling

- `401` → trigger your existing re-auth flow.
- Per-row `error` on the exchanges endpoint is a soft failure — render `--` for that row, leave the others alone.
- Any other non-2xx → show a non-blocking toast and keep last known good data.

---

That's it. Once this is wired up, point me at any FE issues and I can iterate on the backend side.
