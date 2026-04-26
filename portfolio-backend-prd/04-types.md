# 04 — TypeScript Types

These are the request/response types the FE expects. They are the canonical contract — the FE will copy these verbatim into its API service layer once the endpoints are live.

## Common

```ts
// Lowercase venue identifier, matches existing convention.
export type VenueKey = 'hyperliquid' | 'pacifica' | 'backpack' | 'lighter';
```

## 3.1 Performance stats

```ts
export type PerformanceTimeframe = 'day' | 'week' | 'month' | 'all';

export interface PerformanceBucket {
  /** Total notional traded in USD during this bucket. */
  volumeUsd: number;
  /** Number of strategies opened during this bucket. */
  strategiesOpened: number;
  /** Realized + unrealized + funding PnL in USD during this bucket. May be negative. */
  pnlUsd: number;
}

export interface PerformanceResponse {
  day: PerformanceBucket;
  week: PerformanceBucket;
  month: PerformanceBucket;
  all: PerformanceBucket;
}
```

## 3.2 PnL chart

```ts
export interface PnlChartPoint {
  /** ISO 8601 UTC, bucket-end time. */
  timestamp: string;
  /** Cumulative PnL within the window, starts at 0 at rangeStart. */
  cumulativePnlUsd: number;
}

export interface PnlChartResponse {
  timeframe: PerformanceTimeframe;
  /** ISO 8601 UTC */
  rangeStart: string;
  /** ISO 8601 UTC */
  rangeEnd: string;
  points: PnlChartPoint[];
}

export interface PnlChartQuery {
  timeframe: PerformanceTimeframe;
  /** IANA timezone, default 'UTC'. */
  tz?: string;
}
```

## 3.3 Exchanges

```ts
export interface ExchangeRow {
  venue: VenueKey;
  displayName: string;
  connected: boolean;
  /** null when connected = false, or when upstream is unavailable. */
  availableBalanceUsd: number | null;
  /** null when connected = false, or when upstream is unavailable. */
  totalEquityUsd: number | null;
  /** Short error code explaining why values are null; null on success. */
  error: string | null;
}

export interface ExchangesResponse {
  exchanges: ExchangeRow[];
  totals: {
    availableBalanceUsd: number;
    totalEquityUsd: number;
  };
}
```

## Error envelope (shared)

```ts
export interface ApiErrorBody {
  message: string;
}
```
