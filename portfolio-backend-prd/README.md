# Portfolio Page — Backend PRD

This folder is a self-contained handoff to the backend team. It specifies the API endpoints needed to power the `/portfolio` page in the Nuke web app. **You do not need access to the frontend repo to act on this** — all relevant context (page UI, existing API conventions, types) is inlined in the files below.

## Files

1. **[01-page-overview.md](01-page-overview.md)** — What the portfolio page renders, section by section. Includes the actual UI structure and the placeholder data the page currently uses, so you understand exactly what the new endpoints need to populate.
2. **[02-existing-api-context.md](02-existing-api-context.md)** — The existing API conventions in this product (base URL, auth, route prefixes, error handling, venue keys). New endpoints must align with these.
3. **[03-endpoints.md](03-endpoints.md)** — The three new endpoints required, with full request/response specs.
4. **[04-types.md](04-types.md)** — TypeScript types the FE expects, ready to copy into the FE codebase once endpoints are live.
5. **[05-open-questions.md](05-open-questions.md)** — Decisions the backend team needs to make before/during implementation, and questions to resolve with product.

## TL;DR

The portfolio page has two sections:

- **Performance** — four timeframe tabs (Day / Week / Month / All), each showing Volume, Strategies Opened, PnL, plus a PnL time-series line chart.
- **Exchanges** — five cards: Hyperliquid, Backpack, Pacifica, trade[xyz], and an aggregated "All Exchanges" total. Each card shows Available Balance and Total Equity.

We need three endpoints:

| Endpoint | Purpose |
|---|---|
| `GET /aggregated/portfolio/performance/{evm}/{sol}` | Volume, strategies opened, PnL for all four timeframes |
| `GET /aggregated/portfolio/pnl-chart/{evm}/{sol}?timeframe=...` | Time-series points for the PnL chart |
| `GET /aggregated/portfolio/exchanges/{evm}/{sol}` | Per-exchange balance + equity, plus aggregated total |
