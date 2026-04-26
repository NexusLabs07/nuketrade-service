# 05 — Open Questions

Decisions to make before/during implementation. Each item lists the question, the FE's recommendation, and where the resolution should be reflected.

## Q1 — Definition of `pnlUsd`

**Question.** What does `pnlUsd` include?
- Realized PnL only?
- Realized + unrealized?
- Realized + unrealized + funding?

**FE recommendation.** Realized + unrealized + funding. This matches `totalPnl` in the existing positions endpoint and is what users expect when they see "PnL" on a portfolio screen.

**Affects.** [03-endpoints.md](03-endpoints.md) §3.1 and §3.2.

## Q2 — Definition of `strategiesOpened`

**Question.** Is "Strategies Opened" the count of:
- Hedge intents created (`/hedge-intents` POSTs)?
- Cross-venue arbitrage *positions* successfully opened?

These diverge when an intent fails partway (intent created, but no position opened on one or both legs).

**FE recommendation.** Count successfully opened cross-venue positions. Failed intents shouldn't inflate the metric.

**Affects.** [03-endpoints.md](03-endpoints.md) §3.1.

## Q3 — Day / week / month bucket boundaries

**Question.** Are the timeframes:
- Rolling windows (last 24h, last 7d, last 30d), or
- Calendar-aligned in the user's timezone (today, this ISO week, this calendar month)?

**FE recommendation.** Rolling windows. Simpler, matches the chart endpoint, avoids midnight/timezone edge cases.

**Affects.** [03-endpoints.md](03-endpoints.md) §3.1 and §3.2. Whichever is chosen, document it in the API spec.

## Q4 — "trade[xyz]" venue

**Question.** The portfolio page mockup shows a fifth exchange labeled "trade[xyz]" with a "T" monogram. Is this:
- A real upcoming venue that needs a venue key (e.g. `tradexyz`)?
- A placeholder / design artifact that should be removed from the FE?

**FE recommendation.** Confirm with product. Until confirmed, the API returns the four real venues; the FE can keep "trade[xyz]" as a placeholder card or drop it.

**Affects.** [02-existing-api-context.md](02-existing-api-context.md) (venue keys list), [03-endpoints.md](03-endpoints.md) §3.3 (exchanges array).

## Q5 — Disconnected venues in the response

**Question.** Should the exchanges endpoint:
- (A) Always return all supported venues, with `connected: false` for ones the user hasn't set up, or
- (B) Only return connected venues, and let the FE show a separate "Connect an exchange" CTA for the rest?

**FE recommendation.** (A) — always return all venues. The current FE design renders all four cards regardless, with `--` for unconnected ones. Option (A) keeps the FE simpler and lets us add a connect CTA later without changing the API.

**Affects.** [03-endpoints.md](03-endpoints.md) §3.3.

## Q6 — Auth on portfolio endpoints

**Question.** Currently planned as public GETs (user identity in path params), matching `/aggregated/open-positions/...`. Should these instead require a JWT?

**FE recommendation.** Match existing aggregated endpoints (no JWT). If the team prefers JWT-gating, the FE can attach the bearer token — let us know and we'll update the API client.

**Affects.** [02-existing-api-context.md](02-existing-api-context.md), [03-endpoints.md](03-endpoints.md).

## Q7 — Numeric precision

**Question.** What precision should USD numerics be returned at? The FE will format to 2 decimal places by default for display.

**FE recommendation.** Return raw numbers (e.g. `1234.56789`); FE rounds for display. For very small values (e.g. funding sub-cent), preserving precision in the API matters for accurate aggregation.

**Affects.** All endpoints in [03-endpoints.md](03-endpoints.md).

## Q8 — Time series timezone for X-axis labels

**Question.** The PnL chart's X-axis labels are formatted on the FE. Should the API consider the user's timezone when bucketing chart points (so a "day" chart shows the user's local day), or always bucket in UTC and let the FE re-label?

**FE recommendation.** Accept an optional `tz` query param (default `UTC`). Bucket on the server in the requested tz so the day boundary is consistent.

**Affects.** [03-endpoints.md](03-endpoints.md) §3.2.

## Q9 — Future: realtime updates

**Question.** The portfolio page polls today. Is there a plan to push portfolio updates over WebSocket / SSE?

**FE recommendation.** Out of scope for v1 — implement the REST endpoints as specified. If/when a realtime channel is added, the FE will subscribe instead of polling.

**Affects.** Future work; flagged here so it's not forgotten.
