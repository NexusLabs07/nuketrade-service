# Portfolio Backend — Frontend Handoff

The 3 portfolio endpoints from the PRD are live. This folder explains what shipped and what the FE needs to do.

## Files

1. **[01-endpoints.md](01-endpoints.md)** — the 3 endpoints with example requests + responses.
2. **[02-types.md](02-types.md)** — TypeScript types to drop into the FE.
3. **[03-caveats.md](03-caveats.md)** — known limits the FE should handle (Backpack stub, Lighter has no historical buckets, etc.).
4. **[04-fe-checklist.md](04-fe-checklist.md)** — what the FE engineer needs to wire up.

## TL;DR

| Endpoint | Auth | What it does |
|---|---|---|
| `GET /aggregated/portfolio/performance/{evm}/{sol}` | JWT | Volume + PnL + strategiesOpened for Day/Week/Month/All |
| `GET /aggregated/portfolio/pnl-chart/{evm}/{sol}?timeframe=day\|week\|month\|all` | JWT | Time-series points for the PnL chart |
| `GET /aggregated/portfolio/exchanges/{evm}/{sol}` | JWT | Per-venue balance + equity, plus aggregated totals |

**Auth changed from PRD**: these endpoints **require JWT** (`Authorization: Bearer <token>`) — different from `/aggregated/open-positions/...` which is public. The FE client must attach the bearer token.

**Venues returned**: `hyperliquid`, `backpack`, `pacifica`, `lighter`. The mock card "trade[xyz]" was dropped.
