# 02 — TypeScript Types

Drop these into the FE's API service layer. They match exactly what the server returns.

```ts
// Lowercase venue identifier — matches existing convention.
export type VenueKey = 'hyperliquid' | 'pacifica' | 'backpack' | 'lighter';

export type PerformanceTimeframe = 'day' | 'week' | 'month' | 'all';

// ---------- 1. Performance ----------

export interface PerformanceBucket {
  /** Sum of |size × price| across fills in this bucket, in USD. */
  volumeUsd: number;
  /** Count of hedge_intents with status ACTIVE | CANCELLING | CANCELLED in the window. */
  strategiesOpened: number;
  /** Realized PnL on fills in the window, USD. May be negative. */
  pnlUsd: number;
}

export interface PerformanceResponse {
  day: PerformanceBucket;
  week: PerformanceBucket;
  month: PerformanceBucket;
  all: PerformanceBucket;
}

// ---------- 2. PnL chart ----------

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
  /** IANA timezone — accepted but currently ignored server-side. */
  tz?: string;
}

// ---------- 3. Exchanges ----------

export interface ExchangeRow {
  venue: VenueKey;
  displayName: string;
  connected: boolean;
  /** null when connected = false, or when upstream is unavailable. */
  availableBalanceUsd: number | null;
  /** null when connected = false, or when upstream is unavailable. */
  totalEquityUsd: number | null;
  /** Short code explaining why values are null; null on success.
   *  Known codes: "upstream_unavailable", "not_implemented" (Backpack). */
  error: string | null;
}

export interface ExchangesResponse {
  exchanges: ExchangeRow[];
  totals: {
    availableBalanceUsd: number;
    totalEquityUsd: number;
  };
}

// ---------- Errors ----------

export interface ApiErrorBody {
  error: string;
  message: string;
  details?: string[];
}
```
