# 01 — Portfolio Page Overview

This document describes exactly what the `/portfolio` page renders today, so you understand what data the new endpoints need to populate. The page is currently wired to placeholder/static data. Below is the FE structure and the data shapes it consumes — these directly map to what the API needs to return.

## Page layout

The page has two stacked sections inside a centered container (max-width 1560px):

```
┌───────────────────────────────────────────────────────────────┐
│  PERFORMANCE                                                  │
│  ┌──────────────┐  ┌─────────────────────────────────────┐  │
│  │ [Day] [Wk]   │  │                                     │  │
│  │ [Mo]  [All]  │  │      PnL line chart                 │  │
│  ├──────────────┤  │      (X axis: time, Y axis: $)      │  │
│  │ Volume       │  │                                     │  │
│  │ $0.00        │  │                                     │  │
│  ├──────────────┤  │                                     │  │
│  │ Strategies   │  │                                     │  │
│  │ Opened       │  │                                     │  │
│  │ 0            │  │                                     │  │
│  ├──────────────┤  │                                     │  │
│  │ PnL          │  │                                     │  │
│  │ $0           │  │                                     │  │
│  └──────────────┘  └─────────────────────────────────────┘  │
│                                                               │
│  EXCHANGES                                                    │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌────────┐ │
│  │Hyperliq.│ │Backpack │ │Pacifica │ │trade[xy]│ │All Exch│ │
│  │  HL     │ │   BP    │ │   Pa    │ │    T    │ │        │ │
│  ├─────────┤ ├─────────┤ ├─────────┤ ├─────────┤ ├────────┤ │
│  │Avail Bal│ │Avail Bal│ │Avail Bal│ │Avail Bal│ │Avail Bal│ │
│  │   --    │ │   --    │ │   --    │ │   --    │ │ $0.00  │ │
│  ├─────────┤ ├─────────┤ ├─────────┤ ├─────────┤ ├────────┤ │
│  │Tot Equity│ │Tot Eq.  │ │Tot Eq.  │ │Tot Eq.  │ │Tot Eq. │ │
│  │   --    │ │   --    │ │   --    │ │   --    │ │ $0.00  │ │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘ └────────┘ │
└───────────────────────────────────────────────────────────────┘
```

## Section 1 — Performance

### Timeframe tabs
Four tabs control which stats and chart data are shown:

```
Day | Week | Month | All
```

### Stat cards (left column)
Three cards stacked vertically. The third (PnL) is visually highlighted.

For each timeframe, the FE currently consumes this shape:

```ts
{
  volume: string;            // e.g. "$1,234.56"
  strategiesOpened: string;  // e.g. "3"
  pnl: string;               // e.g. "+$12.40"
}
```

The FE preformats these as strings today. **For the new API, return numbers — the FE will format them.** This makes it possible to do client-side aggregation and consistent formatting.

### PnL chart (right column)
A smooth-curve line chart with a soft area gradient underneath. Y axis shows USD values; X axis shows time labels appropriate for the active timeframe (e.g. hourly labels for "Day", weekday labels for "Week").

The FE today hard-codes a list of `(x, y)` points. Replace with a time-series array fetched from the API per timeframe selection.

## Section 2 — Exchanges

Five cards rendered in a horizontal grid. The first four are individual exchanges; the fifth is an aggregate.

### Per-exchange card
The FE today consumes:

```ts
{
  name: string;             // "Hyperliquid"
  mark: string | null;      // logo monogram, e.g. "HL" — null for the aggregate card
  availableBalance: string; // "$1,230.45" or "--" if no data
  totalEquity: string;      // "$1,450.99" or "--"
  highlighted?: boolean;    // true for the "All Exchanges" aggregate card
}
```

Currently rendered exchanges (in this order):

1. **Hyperliquid** (mark: `HL`)
2. **Backpack** (mark: `BP`)
3. **Pacifica** (mark: `Pa`)
4. **trade[xyz]** (mark: `T`) — *unclear whether this is a real upcoming venue or a placeholder; flagged in [05-open-questions.md](05-open-questions.md)*
5. **All Exchanges** (no mark, highlighted, sums of the others)

Empty state: when a user has no data for a given exchange, the FE renders `--`. The API should return numeric values when known and `null` when unknown — the FE handles the `--` rendering.

## Where this data comes from today (placeholders)

Just so you can see the full surface, here is the static data the FE currently uses. **All of this becomes API-driven** once the new endpoints are built.

```ts
// Performance stats — same zeros for every timeframe today
{
  Day:   { volume: '$0.00', strategiesOpened: '0', pnl: '$0' },
  Week:  { volume: '$0.00', strategiesOpened: '0', pnl: '$0' },
  Month: { volume: '$0.00', strategiesOpened: '0', pnl: '$0' },
  All:   { volume: '$0.00', strategiesOpened: '0', pnl: '$0' },
}

// Exchange cards
[
  { name: 'Hyperliquid',   mark: 'HL', availableBalance: '--',    totalEquity: '--' },
  { name: 'Backpack',      mark: 'BP', availableBalance: '--',    totalEquity: '--' },
  { name: 'Pacifica',      mark: 'Pa', availableBalance: '--',    totalEquity: '--' },
  { name: 'trade[xyz]',    mark: 'T',  availableBalance: '--',    totalEquity: '--' },
  { name: 'All Exchanges', mark: null, availableBalance: '$0.00', totalEquity: '$0.00', highlighted: true },
]

// PnL chart — currently hard-coded points, will become API-driven time series
```

## Behavior summary

| Interaction | Effect |
|---|---|
| Page load | Fetch performance stats (all 4 timeframes), PnL chart (default tab = Day), and exchanges in parallel. |
| Switching timeframe tab | Update visible stats from already-fetched data; refetch PnL chart for the new timeframe. |
| Empty / disconnected venue | Show `--` for available balance and total equity. |
| User has no activity at all | Show `$0.00` / `0` for performance, `--` for exchanges (since no exchange is connected). |

This page is read-only — there are no write/mutation endpoints needed.
