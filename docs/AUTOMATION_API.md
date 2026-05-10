# Automation API (Rust) — Frontend Integration Guide

This document defines the **public** automation API exposed by the Rust service for frontend usage.

- **Base path**: `/automation`
- **Audience**: Frontend / product engineers
- **Execution**: *Not* done by the FE. Execution is handled by the external Node executor via `/internal/automation/*` (documented briefly at the end).

---

## Auth

### Normal mode (production)

Send a user JWT:

- Header: `Authorization: Bearer <USER_JWT>`

### Local testing mode (auth disabled for automation only)

If Rust is started with:

- `DISABLE_AUTOMATION_AUTH=1` (or `true`)

Then `/automation/*` does **not** require a JWT, but you must send:

- Header: `X-User-Id: <uuid>`

If you omit `X-User-Id`, Rust returns 401.

---

## Units & conventions (important)

### APR thresholds

All threshold inputs are interpreted as **annualized APR percent**:

- `minAprToEnter`: `15` means **15% APR**, *not* 15 bps
- `exitIfAprBelow`: `2` means **2% APR**

If your UI stores thresholds in **bps**, convert:

\[
\text{aprPercent} = \frac{\text{aprBps}}{100}
\]

Examples:

- `1500 bps → 15`
- `200 bps → 2`

### Exchange ids

Use lowercase exchange ids:

- `hyperliquid`
- `pacifica`

---

## 1) Config

### 1.1 GET `/automation/config`

Returns the user’s config (or defaults if none exists).

#### Response: `AutomationConfigResponse`

```ts
export type AutomationAprMode = "NET" | "SEVEN_D";

export type AutomationConfigResponse = {
  userId: string; // uuid

  aprMode: AutomationAprMode;

  // annualized APR percent (not bps)
  minAprToEnter: number;
  exitIfAprBelow: number;

  rebalanceToBetterPair: boolean;
  minRebalanceImprovementBps: number;
  minTimeBetweenActionsSec: number;
  cooldownAfterErrorSec: number;

  maxPositionSizeUsd: number;
  maxLeverage: number;       // >= 1
  maxActionsPerDay: number;

  excludedAssets: string[];  // symbols (typically uppercase)
  allowedExchanges: string[];// e.g. ["hyperliquid","pacifica"]

  // Node executor constraints (pass-through)
  maxSlippageBps: number;     // default 50
  reduceOnlyOnClose: boolean; // default true

  createdAt: string; // RFC3339
  updatedAt: string; // RFC3339
};
```

---

### 1.2 PUT `/automation/config`

Upserts the user’s automation config.

#### Request: `UpsertAutomationConfigRequest`

```ts
export type UpsertAutomationConfigRequest = {
  // accepts "7D" alias (normalized to "SEVEN_D")
  aprMode: "NET" | "SEVEN_D" | "7D";

  // annualized APR percent (not bps)
  minAprToEnter: number;
  exitIfAprBelow: number;

  rebalanceToBetterPair: boolean;

  // churn control / hysteresis
  minRebalanceImprovementBps: number;
  minTimeBetweenActionsSec: number;

  // cooldown after failure
  cooldownAfterErrorSec: number;

  // sizing limits
  maxPositionSizeUsd: number;
  maxLeverage: number;        // >= 1
  maxActionsPerDay: number;

  excludedAssets: string[];

  // defaults to ["hyperliquid","pacifica"] if omitted
  allowedExchanges?: string[];

  // Node executor constraints (optional)
  maxSlippageBps?: number;      // default 50
  reduceOnlyOnClose?: boolean;  // default true
};
```

#### Response

Same as `AutomationConfigResponse`.

---

## 2) Best pair preview

### GET `/automation/best-pair?mode=NET|SEVEN_D`

Returns a **Recommendation** for UI preview (read-only).

#### Query params

- `mode` (optional): `"NET"` or `"SEVEN_D"`

#### Response: `Recommendation`

```ts
export type RecommendedLeg = {
  exchange: string; // "hyperliquid" | "pacifica" | ...
  side: "LONG" | "SHORT";
  weight: number;  // v1 always 1.0
};

export type LegFundingInput = {
  exchange: string;
  funding: number;
  maxLeverage?: number | null;
};

export type ReferencePrice = {
  symbol: string;
  px: string;         // decimal string
  source: "mark";     // v1 fixed
  tsMs: number;
};

export type Recommendation = {
  decisionId: string;     // uuid
  decisionHash: string;

  asset: string | null;
  legs: RecommendedLeg[];

  metricMode: "NET" | "SEVEN_D";
  metricValueAprPct: number;
  metricValueRaw: number;
  inputs: LegFundingInput[];

  asOfTs: number;       // unix seconds
  asOfBucket: number;   // bucketed for idempotency

  recommendedAction:
    | "OPEN"
    | "HOLD"
    | "CLOSE"
    | "REBALANCE"
    | "EMERGENCY_CLOSE"
    | "NOOP_BELOW_MIN";

  reasons: string[];

  // optional in preview responses
  referencePrice?: ReferencePrice;
};
```

---

## 3) Runs

Runs represent a live automation instance for a user.

### 3.1 POST `/automation/runs`

Creates and activates a new run (fails if the user already has an ACTIVE run).

#### Request: `CreateRunRequest`

```ts
export type CreateRunRequest = {
  targetMarginUsd?: number; // optional
  leverage?: number;        // optional, >= 1
};
```

#### Response: `CreateRunResponse`

```ts
export type CreateRunResponse = {
  runId: string; // uuid
};
```

---

### 3.2 GET `/automation/runs`

Lists runs for the authenticated user.

#### Response: `AutomationRunResponse[]`

---

### 3.3 GET `/automation/runs/{id}`

Fetch a single run.

#### Response: `AutomationRunResponse`

```ts
export type AutomationRunStatus =
  | "DRAFT"
  | "ACTIVE"
  | "PAUSED"
  | "STOPPING"
  | "STOPPED"
  | "FAILED";

export type AutomationRunResponse = {
  id: string;     // uuid
  userId: string; // uuid
  status: AutomationRunStatus;

  // snapshot of config at run start
  aprMode: "NET" | "SEVEN_D";
  minAprToEnter: number;
  exitIfAprBelow: number;
  rebalanceToBetterPair: boolean;
  maxLeverage: number;
  maxActionsPerDay: number;
  configHash: string;

  // runtime state / position tracking
  currentAsset: string | null;
  currentLegs: unknown | null; // JSON array of legs

  targetMarginUsd: number | null;
  leverage: number | null;

  lastDecisionId: string | null; // uuid
  actionsToday: number;

  lastRecommendationAt: string | null; // RFC3339
  lastActionAt: string | null;         // RFC3339
  lastErrorAt: string | null;          // RFC3339
  lastError: string | null;

  // deprecated for automation flow; not populated by v2
  currentHedgeIntentId: string | null;

  createdAt: string; // RFC3339
  updatedAt: string; // RFC3339
};
```

---

### 3.4 Run controls

All return:

```ts
export type ActionResponse = { status: "ok"; message: string };
```

Endpoints:

- POST `/automation/runs/{id}/pause` (ACTIVE → PAUSED)
- POST `/automation/runs/{id}/resume` (PAUSED → ACTIVE)
- POST `/automation/runs/{id}/stop` (ACTIVE/PAUSED → STOPPING)
- POST `/automation/runs/{id}/restart` (STOPPED/FAILED → creates a new ACTIVE run)

Note: when a run is `STOPPING` and still has a position, the internal executor
pipeline will emit an `EMERGENCY_CLOSE` intent for the **current pair**.

---

## 4) Actions audit log

### GET `/automation/runs/{id}/actions`

Returns the raw `automation_actions` rows (append-only audit log).

The returned shape is DB-driven and includes `status`, `error`, `result_json`,
lease fields, etc. Use this endpoint for debugging and observability UI.

---

## FE payload mapping (from your current request)

Your current FE body (example):

```json
{
  "exchanges": ["pacifica", "hyperliquid"],
  "minAprBps": 1500,
  "exitAprBps": 200,
  "maxNotionalUsd": "100",
  "maxLeverage": 3,
  "maxActionsPerDay": 20,
  "blocklist": [],
  "closeOnFundingFlip": true
}
```

Map it to Rust `PUT /automation/config` like this:

- `exchanges` → `allowedExchanges`
- `blocklist` → `excludedAssets`
- `minAprBps` → `minAprToEnter = minAprBps / 100`
- `exitAprBps` → `exitIfAprBelow = exitAprBps / 100`
- `maxNotionalUsd` (string) → `maxPositionSizeUsd` (number)
- `closeOnFundingFlip` → **not supported** by Rust config (remove)

Example converted payload:

```json
{
  "aprMode": "NET",
  "minAprToEnter": 15,
  "exitIfAprBelow": 2,
  "rebalanceToBetterPair": true,
  "minRebalanceImprovementBps": 50,
  "minTimeBetweenActionsSec": 300,
  "cooldownAfterErrorSec": 900,
  "maxPositionSizeUsd": 100,
  "maxLeverage": 3,
  "maxActionsPerDay": 20,
  "excludedAssets": [],
  "allowedExchanges": ["pacifica", "hyperliquid"],
  "maxSlippageBps": 50,
  "reduceOnlyOnClose": true
}
```

---

## Internal endpoints (for Node executor; FE should not call)

These are mounted at `/internal/automation/*` and are polled by the external Node executor:

- GET `/internal/automation/intents/due?limit=N`
- POST `/internal/automation/intents/{intentId}/result`

In normal mode these require:

- `Authorization: Bearer <AUTOMATION_INTERNAL_TOKEN>`

In local testing mode (`DISABLE_AUTOMATION_AUTH=1`) they require no auth.

