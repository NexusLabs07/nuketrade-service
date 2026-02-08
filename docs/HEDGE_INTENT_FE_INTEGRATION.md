# Hedge Intent — Frontend Integration Guide

> **Architecture:** Client-Signed, Backend-Orchestrated Saga  
> **Base URL:** `https://api.nuketrade.xyz` (or `http://localhost:3000` for local dev)  
> **Prefix:** `/hedge-intents`

---

## Table of Contents

1. [Core Concept](#1-core-concept)
2. [API Endpoints](#2-api-endpoints)
3. [TypeScript Types](#3-typescript-types)
4. [Status Enums Reference](#4-status-enums-reference)
5. [The Polling Loop (Core Integration)](#5-the-polling-loop)
6. [Action Handlers](#6-action-handlers)
7. [Reporting Results](#7-reporting-results)
8. [Complete Client Implementation](#8-complete-client-implementation)
9. [UI State Mapping](#9-ui-state-mapping)
10. [Error Handling & Retries](#10-error-handling--retries)
11. [Edge Cases](#11-edge-cases)
12. [Sequence Diagrams](#12-sequence-diagrams)

---

## 1. Core Concept

The backend owns a **state machine**. The frontend is a **stateless executor**.

```
User clicks "Open Hedged Position"
  → FE calls POST /hedge-intents (creates the intent)
  → FE enters a polling loop:
      1. GET /hedge-intents/{id}/next-action  →  "What should I do?"
      2. Execute that action (sign tx, bridge, deposit, etc.)
      3. POST /hedge-intents/{id}/action-result  →  "Here's what happened"
      4. Repeat until action = "NOOP"
```

**Key rules:**
- The **backend never signs** anything
- The **frontend never decides** what to do next
- The frontend is **stateless** — if the user refreshes, it just resumes the loop
- Every action is **idempotent** — safe to retry

---

## 2. API Endpoints

### 2.1 Create Hedge Intent

```
POST /hedge-intents
```

Creates a new hedge intent with two legs (one per protocol). The margin is split 50/50 between protocols.

**Request Body:**

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "asset": "BTC",
  "protocols": ["HL", "PACIFICA"],
  "margin_usd": 1000.0,
  "leverage": 5.0,
  "evm_address": "0x1234567890abcdef1234567890abcdef12345678",
  "solana_address": "7nYBm5mK3W5CjHbKsLcD5PxqH4TKZfR5yHv9qT3bPKjE"
}
```

| Field            | Type       | Description                                          |
|------------------|------------|------------------------------------------------------|
| `user_id`        | `UUID`     | The authenticated user's ID                          |
| `asset`          | `string`   | Trading asset (e.g. `"BTC"`, `"ETH"`)                |
| `protocols`      | `string[]` | Exactly 2 different protocols: `"HL"`, `"PACIFICA"`  |
| `margin_usd`     | `number`   | Total margin in USD (split 50/50 across legs)        |
| `leverage`       | `number`   | Leverage multiplier (must be ≥ 1.0)                  |
| `evm_address`    | `string`   | User's EVM wallet address (for HL on Arbitrum)       |
| `solana_address` | `string`   | User's Solana wallet address (for Pacifica)          |

**Response — `200 OK`:**

```json
{
  "hedge_intent_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
}
```

**Error — `400 Bad Request`:**

```json
{
  "error": "parse_error",
  "message": "Invalid value for protocols: Exactly 2 protocols are required"
}
```

**Validation rules:**
- `protocols` must contain exactly 2 entries
- Protocols must be different from each other
- Supported protocols: `"HL"`, `"PACIFICA"`
- `margin_usd` must be > 0
- `leverage` must be ≥ 1.0

---

### 2.2 Get Next Action

```
GET /hedge-intents/{id}/next-action
```

The **core polling endpoint**. Returns the next action the client should execute.

**Response — `200 OK`:**

```json
{
  "action": "BRIDGE_BASE_TO_ARB",
  "leg": "HL",
  "amount_usd": 500.0,
  "params": {
    "origin_chain_id": 8453,
    "destination_chain_id": 42161,
    "origin_currency": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
    "destination_currency": "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
    "user_address": "0x1234...",
    "recipient": "0x1234...",
    "leg_id": "b2c3d4e5-f6a7-8901-bcde-f12345678901"
  }
}
```

| Field        | Type              | Description                                       |
|--------------|-------------------|---------------------------------------------------|
| `action`     | `string`          | The action type to execute (see §4.3)             |
| `leg`        | `string \| null`  | Which protocol leg this action is for             |
| `amount_usd` | `number \| null`  | Amount in USD for this action                     |
| `params`     | `object \| null`  | Action-specific parameters (see §6)               |

**Possible `action` values:**

| Action                 | Meaning                                          |
|------------------------|--------------------------------------------------|
| `BRIDGE_BASE_TO_ARB`   | Bridge USDC from Base → Arbitrum (for HL leg)    |
| `BRIDGE_BASE_TO_SOL`   | Bridge USDC from Base → Solana (for Pacifica)    |
| `DEPOSIT_TO_HL`        | Deposit USDC into Hyperliquid margin account     |
| `DEPOSIT_TO_PACIFICA`  | Deposit USDC into Pacifica margin account        |
| `OPEN_HEDGE_POSITION`  | Open the hedged position on both protocols       |
| `CLOSE_POSITION`       | Close a position (safety mode / unwinding)       |
| `WAIT`                 | Nothing to do right now — poll again after delay |
| `NOOP`                 | Terminal state — stop polling                    |

---

### 2.3 Report Action Result

```
POST /hedge-intents/{id}/action-result
```

After executing an action, the client reports the outcome back to the backend.

**Request Body (for bridge/deposit actions):**

```json
{
  "action": "BRIDGE_BASE_TO_ARB",
  "success": true,
  "tx_hash": "0xabc123...",
  "error": null,
  "leg_results": null
}
```

**Request Body (for OPEN_HEDGE_POSITION — per-leg results):**

```json
{
  "action": "OPEN_HEDGE_POSITION",
  "success": true,
  "tx_hash": null,
  "error": null,
  "leg_results": [
    {
      "protocol": "HL",
      "success": true,
      "tx_hash": "0xdef456...",
      "error": null
    },
    {
      "protocol": "PACIFICA",
      "success": true,
      "tx_hash": "3xYz...",
      "error": null
    }
  ]
}
```

**Request Body (failure):**

```json
{
  "action": "BRIDGE_BASE_TO_ARB",
  "success": false,
  "tx_hash": null,
  "error": "Transaction reverted: insufficient allowance",
  "leg_results": null
}
```

| Field          | Type                    | Description                                         |
|----------------|-------------------------|-----------------------------------------------------|
| `action`       | `string`                | **Must match** the action from `next-action`        |
| `success`      | `boolean`               | Whether the action succeeded                        |
| `tx_hash`      | `string \| null`        | On-chain tx hash if available                       |
| `error`        | `string \| null`        | Error message if failed                             |
| `leg_results`  | `LegResult[] \| null`   | Per-leg results (only for `OPEN_HEDGE_POSITION`)    |

**`LegResult` schema:**

| Field      | Type             | Description                         |
|------------|------------------|-------------------------------------|
| `protocol` | `string`         | `"HL"` or `"PACIFICA"`             |
| `success`  | `boolean`        | Whether this leg's position opened  |
| `tx_hash`  | `string \| null` | Transaction hash                    |
| `error`    | `string \| null` | Error if failed                     |

**Response — `200 OK`:**

```json
{
  "status": "accepted",
  "message": "Action result for BRIDGE_BASE_TO_ARB processed"
}
```

---

### 2.4 Get Hedge Intent Detail

```
GET /hedge-intents/{id}
```

Returns the full intent including all legs. Useful for rendering status UI.

**Response — `200 OK`:**

```json
{
  "intent": {
    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "asset": "BTC",
    "protocol_a": "HL",
    "protocol_b": "PACIFICA",
    "margin_usd": 1000.0,
    "leverage": 5.0,
    "evm_address": "0x1234...",
    "solana_address": "7nYBm...",
    "status": "FUNDING",
    "created_at": "2026-02-07T12:00:00",
    "updated_at": "2026-02-07T12:01:30"
  },
  "legs": [
    {
      "id": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
      "hedge_intent_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "protocol": "HL",
      "chain": "ARB",
      "target_amount_usd": 500.0,
      "funded_amount_usd": 500.0,
      "status": "FUNDED",
      "retry_count": 0,
      "last_error": null,
      "created_at": "2026-02-07T12:00:00",
      "updated_at": "2026-02-07T12:01:00"
    },
    {
      "id": "c3d4e5f6-a7b8-9012-cdef-123456789012",
      "hedge_intent_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "protocol": "PACIFICA",
      "chain": "SOL",
      "target_amount_usd": 500.0,
      "funded_amount_usd": 0.0,
      "status": "BRIDGE_IN_PROGRESS",
      "retry_count": 0,
      "last_error": null,
      "created_at": "2026-02-07T12:00:00",
      "updated_at": "2026-02-07T12:00:30"
    }
  ]
}
```

---

### 2.5 List User's Hedge Intents

```
GET /hedge-intents/user/{user_id}
```

Returns all hedge intents for a user, ordered by most recent first.

**Response — `200 OK`:**

```json
[
  {
    "id": "a1b2c3d4-...",
    "user_id": "550e8400-...",
    "asset": "BTC",
    "protocol_a": "HL",
    "protocol_b": "PACIFICA",
    "margin_usd": 1000.0,
    "leverage": 5.0,
    "evm_address": "0x1234...",
    "solana_address": "7nYBm...",
    "status": "ACTIVE",
    "created_at": "2026-02-07T12:00:00",
    "updated_at": "2026-02-07T12:05:00"
  }
]
```

---

## 3. TypeScript Types

Copy these into your frontend codebase:

```typescript
// ========================= API Types =========================

interface CreateHedgeIntentRequest {
  user_id: string;             // UUID
  asset: string;               // e.g. "BTC"
  protocols: Protocol[];       // exactly 2
  margin_usd: number;
  leverage: number;
  evm_address: string;         // 0x... Ethereum/Arbitrum address
  solana_address: string;      // Base58 Solana address
}

interface CreateHedgeIntentResponse {
  hedge_intent_id: string;     // UUID
}

interface NextActionResponse {
  action: HedgeAction;
  leg: Protocol | null;
  amount_usd: number | null;
  params: Record<string, any> | null;
}

interface ActionResultRequest {
  action: HedgeAction;
  success: boolean;
  tx_hash: string | null;
  error: string | null;
  leg_results: LegResultEntry[] | null;
}

interface LegResultEntry {
  protocol: Protocol;
  success: boolean;
  tx_hash: string | null;
  error: string | null;
}

interface ActionResultResponse {
  status: string;
  message: string;
}

interface HedgeIntentDetail {
  intent: HedgeIntent;
  legs: HedgeLeg[];
}

// ========================= Domain Types =========================

interface HedgeIntent {
  id: string;
  user_id: string;
  asset: string;
  protocol_a: string;
  protocol_b: string;
  margin_usd: number;
  leverage: number;
  evm_address: string;
  solana_address: string;
  status: HedgeIntentStatus;
  created_at: string;
  updated_at: string;
}

interface HedgeLeg {
  id: string;
  hedge_intent_id: string;
  protocol: Protocol;
  chain: Chain;
  target_amount_usd: number;
  funded_amount_usd: number;
  status: HedgeLegStatus;
  retry_count: number;
  last_error: string | null;
  created_at: string;
  updated_at: string;
  /** USDC already in the protocol's margin account (queried on CREATED→FUNDING). */
  existing_margin_usd: number;
  /** USDC on the destination chain wallet, not yet deposited (queried on CREATED→FUNDING). */
  existing_onchain_usd: number;
}

// ========================= Enums =========================

type Protocol = "HL" | "PACIFICA";

type Chain = "ARB" | "SOL";

type HedgeIntentStatus =
  | "CREATED"
  | "FUNDING"
  | "READY"
  | "OPENING"
  | "ACTIVE"
  | "FAILED"
  | "CANCELLING"
  | "CANCELLED";

type HedgeLegStatus =
  | "PENDING"
  | "BRIDGE_IN_PROGRESS"
  | "BRIDGE_CONFIRMED"
  | "DEPOSIT_IN_PROGRESS"
  | "FUNDED"
  | "OPENING_POSITION"
  | "ACTIVE"
  | "FAILED"
  | "CLOSING"
  | "CLOSED";

type HedgeAction =
  | "BRIDGE_BASE_TO_ARB"
  | "BRIDGE_BASE_TO_SOL"
  | "DEPOSIT_TO_HL"
  | "DEPOSIT_TO_PACIFICA"
  | "OPEN_HEDGE_POSITION"
  | "CLOSE_POSITION"
  | "WAIT"
  | "NOOP";
```

---

## 4. Status Enums Reference

### 4.1 HedgeIntentStatus — Lifecycle

```
CREATED → FUNDING → READY → OPENING → ACTIVE
                ↓                ↓
              FAILED ←←←←←←←← FAILED
                ↓                ↓
            CANCELLING      (safety mode)
                ↓
            CANCELLED
```

| Status        | Meaning                                                      |
|---------------|--------------------------------------------------------------|
| `CREATED`     | Intent created, no execution started                         |
| `FUNDING`     | Bridges and deposits are in progress                         |
| `READY`       | Both legs funded, ready to open positions                    |
| `OPENING`     | Position open transactions submitted                         |
| `ACTIVE`      | Hedge is live — both positions open                          |
| `FAILED`      | Unrecoverable failure or partial execution (safety mode)     |
| `CANCELLING`  | Unwinding/closing active positions                           |
| `CANCELLED`   | Fully unwound and stopped                                    |

### 4.2 HedgeLegStatus — Per-Leg Lifecycle

```
PENDING → BRIDGE_IN_PROGRESS → BRIDGE_CONFIRMED → DEPOSIT_IN_PROGRESS → FUNDED
                                                                           ↓
                                                               OPENING_POSITION → ACTIVE
                                                                                    ↓
Any state can → FAILED                                                          CLOSING → CLOSED
```

| Status                | Meaning                                    |
|-----------------------|--------------------------------------------|
| `PENDING`             | Waiting to start                           |
| `BRIDGE_IN_PROGRESS`  | Bridge tx submitted, waiting confirmation  |
| `BRIDGE_CONFIRMED`    | Bridge successful, ready for deposit       |
| `DEPOSIT_IN_PROGRESS` | Deposit tx submitted                       |
| `FUNDED`              | Deposit confirmed, margin available        |
| `OPENING_POSITION`    | Position open tx submitted                 |
| `ACTIVE`              | Position is live on this protocol          |
| `FAILED`              | Action failed (may have retries left)      |
| `CLOSING`             | Closing position (safety mode)             |
| `CLOSED`              | Position closed                            |

### 4.3 Actions

| Action                 | Trigger Condition                                           | Client Must Do                                      |
|------------------------|-------------------------------------------------------------|-----------------------------------------------------|
| `BRIDGE_BASE_TO_ARB`   | HL leg is PENDING                                           | Bridge USDC: Base (8453) → Arbitrum (42161)         |
| `BRIDGE_BASE_TO_SOL`   | PACIFICA leg is PENDING                                     | Bridge USDC: Base (8453) → Solana (792703809)       |
| `DEPOSIT_TO_HL`        | HL leg is BRIDGE_CONFIRMED                                  | Deposit USDC into Hyperliquid margin                |
| `DEPOSIT_TO_PACIFICA`  | PACIFICA leg is BRIDGE_CONFIRMED                            | Deposit USDC into Pacifica margin                   |
| `OPEN_HEDGE_POSITION`  | Both legs FUNDED, intent is READY                           | Open long on one protocol, short on the other       |
| `CLOSE_POSITION`       | Intent FAILED with one active leg (safety mode)             | Close the active leg to prevent exposure             |
| `WAIT`                 | An action is in-progress, nothing to do yet                 | Poll again after a delay                            |
| `NOOP`                 | Intent is in a terminal state (ACTIVE/CANCELLED/FAILED)     | Stop polling                                        |

---

## 5. The Polling Loop

This is the **heart of the FE integration**. After creating an intent, the FE enters a simple poll-execute-report loop.

### 5.1 Pseudocode

```
function executeHedgeIntent(intentId):
    while true:
        nextAction = GET /hedge-intents/{intentId}/next-action

        switch nextAction.action:
            case "WAIT":
                sleep(3000)     // poll again in 3s
                continue

            case "NOOP":
                return          // terminal — stop polling

            case "BRIDGE_BASE_TO_ARB":
            case "BRIDGE_BASE_TO_SOL":
                result = await executeBridge(nextAction)
                POST /hedge-intents/{intentId}/action-result  ← result
                continue

            case "DEPOSIT_TO_HL":
            case "DEPOSIT_TO_PACIFICA":
                result = await executeDeposit(nextAction)
                POST /hedge-intents/{intentId}/action-result  ← result
                continue

            case "OPEN_HEDGE_POSITION":
                result = await executeHedgeOpen(nextAction)
                POST /hedge-intents/{intentId}/action-result  ← result
                continue

            case "CLOSE_POSITION":
                result = await executeClose(nextAction)
                POST /hedge-intents/{intentId}/action-result  ← result
                continue
```

### 5.2 Recommended Polling Intervals

| Condition                           | Interval |
|-------------------------------------|----------|
| Action is `WAIT`                    | 3s       |
| After reporting a result            | 500ms    |
| During bridge (can be slow)         | 5s       |
| General `next-action` check         | 2s       |

### 5.3 Resumability

If the user **refreshes the page** or **closes and reopens the tab**:

1. Load the user's intents: `GET /hedge-intents/user/{userId}`
2. Find any intent with status `FUNDING`, `READY`, `OPENING`, or `FAILED`
3. Re-enter the polling loop for that intent

The backend tracks all state — the FE just picks up where it left off.

---

## 6. Action Handlers

### 6.1 BRIDGE_BASE_TO_ARB

Bridge USDC from Base → Arbitrum for the Hyperliquid leg.

**`params` shape:**

```json
{
  "origin_chain_id": 8453,
  "destination_chain_id": 42161,
  "origin_currency": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
  "destination_currency": "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
  "user_address": "0x1234...",
  "recipient": "0x1234...",
  "leg_id": "b2c3d4e5-..."
}
```

**What to do:**
1. Use your bridge SDK (e.g. Relay, LiFi, Socket) with the params above
2. Sign the bridge transaction with Turnkey (user's EVM signer)
3. Submit the transaction
4. Wait for confirmation (or return immediately and let bridge confirm async)

**Report result:**
```json
{
  "action": "BRIDGE_BASE_TO_ARB",
  "success": true,
  "tx_hash": "0x...",
  "error": null,
  "leg_results": null
}
```

---

### 6.2 BRIDGE_BASE_TO_SOL

Bridge USDC from Base → Solana for the Pacifica leg.

**`params` shape:**

```json
{
  "origin_chain_id": 8453,
  "destination_chain_id": 792703809,
  "origin_currency": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
  "destination_currency": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  "user_address": "7nYBm...",
  "recipient": "7nYBm...",
  "leg_id": "c3d4e5f6-..."
}
```

**What to do:**
1. Use bridge SDK to initiate Base → Solana USDC transfer
2. Sign with Turnkey (EVM signer for the Base side; some bridges handle the Solana side automatically)
3. Submit and optionally wait for confirmation

---

### 6.3 DEPOSIT_TO_HL

Deposit USDC into Hyperliquid's on-chain margin account on Arbitrum.

**`params` shape:**

```json
{
  "protocol": "HL",
  "chain": "ARB",
  "user_address": "0x1234...",
  "amount_usd": 500.0,
  "leg_id": "b2c3d4e5-..."
}
```

**What to do:**
1. Approve USDC spend to Hyperliquid's deposit contract (if not already approved)
2. Call the Hyperliquid deposit function
3. Sign with Turnkey (EVM signer)
4. Submit and confirm

---

### 6.4 DEPOSIT_TO_PACIFICA

Deposit USDC into Pacifica's on-chain margin account on Solana.

**`params` shape:**

```json
{
  "protocol": "PACIFICA",
  "chain": "SOL",
  "user_address": "7nYBm...",
  "amount_usd": 500.0,
  "leg_id": "c3d4e5f6-..."
}
```

**What to do:**
1. Build the Pacifica deposit instruction
2. Sign with Turnkey (Solana signer)
3. Submit and confirm

---

### 6.5 OPEN_HEDGE_POSITION

Open delta-neutral hedge positions on both protocols simultaneously.

**`params` shape:**

```json
{
  "asset": "BTC",
  "leverage": 5.0,
  "effective_margin_usd": 500.0,
  "legs": [
    {
      "protocol": "HL",
      "chain": "ARB",
      "funded_amount_usd": 500.0
    },
    {
      "protocol": "PACIFICA",
      "chain": "SOL",
      "funded_amount_usd": 500.0
    }
  ]
}
```

**Key:** `effective_margin_usd` is the **minimum** of the funded amounts across both legs. This ensures the hedge is **delta-neutral** — you don't open a bigger position on one side.

**What to do:**
1. Open a **long** position on one protocol (e.g. HL)
2. Open a **short** position on the other (e.g. Pacifica)
3. Both positions should use `effective_margin_usd × leverage` as notional size
4. Sign both transactions with Turnkey (EVM for HL, Solana for Pacifica)
5. **Report per-leg results** (critical for safety mode)

**Report result (per-leg — RECOMMENDED):**
```json
{
  "action": "OPEN_HEDGE_POSITION",
  "success": true,
  "tx_hash": null,
  "error": null,
  "leg_results": [
    {
      "protocol": "HL",
      "success": true,
      "tx_hash": "0xabc...",
      "error": null
    },
    {
      "protocol": "PACIFICA",
      "success": true,
      "tx_hash": "3xYz...",
      "error": null
    }
  ]
}
```

> **⚠️ IMPORTANT:** Always use `leg_results` for `OPEN_HEDGE_POSITION`. If one leg succeeds and the other fails, the backend activates **Safety Mode** to close the opened leg automatically.

---

### 6.6 CLOSE_POSITION (Safety Mode)

This action appears when the intent is in `FAILED` state and one leg has an active position. The backend needs the FE to close it to prevent **directional exposure**.

**`params` shape:**

```json
{
  "asset": "BTC",
  "protocol": "HL",
  "chain": "ARB",
  "reason": "safety_mode_partial_hedge"
}
```

**What to do:**
1. Close the position on the specified protocol
2. Sign with appropriate signer (EVM for HL, Solana for Pacifica)
3. Report result

**Report result:**
```json
{
  "action": "CLOSE_POSITION",
  "success": true,
  "tx_hash": "0xdef...",
  "error": null,
  "leg_results": null
}
```

---

## 7. Reporting Results

### 7.1 Always Report — Even Failures

The backend tracks retries. Each leg gets up to **3 retries** (`MAX_RETRIES = 3`) before being marked as permanently failed. Always report failures:

```json
{
  "action": "BRIDGE_BASE_TO_ARB",
  "success": false,
  "tx_hash": null,
  "error": "Transaction reverted: ERC20: transfer amount exceeds balance",
  "leg_results": null
}
```

### 7.2 Report Actions Must Match

The `action` field in the report **must match exactly** the `action` field from the `next-action` response. Mismatches return a `400` error.

### 7.3 What Happens On Failure

| Retry Count | Behavior                                                     |
|-------------|--------------------------------------------------------------|
| 1st failure | Backend increments retry_count, next `next-action` returns the same action again |
| 2nd failure | Same — retried again                                        |
| 3rd failure | Leg marked as `FAILED`, intent transitions to `FAILED`      |
|             | If other leg has active position → Safety Mode activates    |

---

## 8. Complete Client Implementation

```typescript
// ========================= API Client =========================

const API_BASE = "https://api.nuketrade.xyz"; // or your backend URL

async function createHedgeIntent(
  req: CreateHedgeIntentRequest
): Promise<string> {
  const res = await fetch(`${API_BASE}/hedge-intents`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  if (!res.ok) throw new Error(await res.text());
  const data: CreateHedgeIntentResponse = await res.json();
  return data.hedge_intent_id;
}

async function getNextAction(intentId: string): Promise<NextActionResponse> {
  const res = await fetch(`${API_BASE}/hedge-intents/${intentId}/next-action`);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

async function reportActionResult(
  intentId: string,
  result: ActionResultRequest
): Promise<void> {
  const res = await fetch(
    `${API_BASE}/hedge-intents/${intentId}/action-result`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(result),
    }
  );
  if (!res.ok) throw new Error(await res.text());
}

async function getHedgeIntentDetail(
  intentId: string
): Promise<HedgeIntentDetail> {
  const res = await fetch(`${API_BASE}/hedge-intents/${intentId}`);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

async function getUserIntents(userId: string): Promise<HedgeIntent[]> {
  const res = await fetch(`${API_BASE}/hedge-intents/user/${userId}`);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

// ========================= Execution Engine =========================

interface ExecutionCallbacks {
  onStatusChange: (status: string, detail?: string) => void;
  onError: (error: string) => void;
  onComplete: (intentId: string) => void;
  
  // These must be implemented by the FE — they sign + submit transactions
  executeBridge: (params: Record<string, any>, amountUsd: number) => Promise<{ txHash: string }>;
  executeDeposit: (protocol: Protocol, params: Record<string, any>, amountUsd: number) => Promise<{ txHash: string }>;
  executeOpenPosition: (params: Record<string, any>) => Promise<LegResultEntry[]>;
  executeClosePosition: (protocol: Protocol, params: Record<string, any>) => Promise<{ txHash: string }>;
}

async function executeHedgeIntent(
  intentId: string,
  callbacks: ExecutionCallbacks
): Promise<void> {
  const POLL_INTERVAL_WAIT = 3000;
  const POLL_INTERVAL_FAST = 500;
  
  while (true) {
    let nextAction: NextActionResponse;
    
    try {
      nextAction = await getNextAction(intentId);
    } catch (err) {
      callbacks.onError(`Failed to fetch next action: ${err}`);
      await sleep(POLL_INTERVAL_WAIT);
      continue;
    }

    callbacks.onStatusChange(`Action: ${nextAction.action}`, 
      nextAction.leg ? `Leg: ${nextAction.leg}` : undefined);

    switch (nextAction.action) {
      // ─── Terminal States ─────────────────────────────
      case "NOOP":
        // Check final state to determine if success or failure
        const detail = await getHedgeIntentDetail(intentId);
        if (detail.intent.status === "ACTIVE") {
          callbacks.onComplete(intentId);
        } else {
          callbacks.onError(`Intent ended in ${detail.intent.status} state`);
        }
        return;

      // ─── Wait ────────────────────────────────────────
      case "WAIT":
        await sleep(POLL_INTERVAL_WAIT);
        continue;

      // ─── Bridge Actions ──────────────────────────────
      case "BRIDGE_BASE_TO_ARB":
      case "BRIDGE_BASE_TO_SOL": {
        let result: ActionResultRequest;
        try {
          const { txHash } = await callbacks.executeBridge(
            nextAction.params!,
            nextAction.amount_usd!
          );
          result = {
            action: nextAction.action,
            success: true,
            tx_hash: txHash,
            error: null,
            leg_results: null,
          };
        } catch (err: any) {
          result = {
            action: nextAction.action,
            success: false,
            tx_hash: null,
            error: err.message ?? String(err),
            leg_results: null,
          };
        }
        await reportActionResult(intentId, result);
        await sleep(POLL_INTERVAL_FAST);
        continue;
      }

      // ─── Deposit Actions ─────────────────────────────
      case "DEPOSIT_TO_HL":
      case "DEPOSIT_TO_PACIFICA": {
        let result: ActionResultRequest;
        const protocol = nextAction.leg! as Protocol;
        try {
          const { txHash } = await callbacks.executeDeposit(
            protocol,
            nextAction.params!,
            nextAction.amount_usd!
          );
          result = {
            action: nextAction.action,
            success: true,
            tx_hash: txHash,
            error: null,
            leg_results: null,
          };
        } catch (err: any) {
          result = {
            action: nextAction.action,
            success: false,
            tx_hash: null,
            error: err.message ?? String(err),
            leg_results: null,
          };
        }
        await reportActionResult(intentId, result);
        await sleep(POLL_INTERVAL_FAST);
        continue;
      }

      // ─── Open Hedge Position ─────────────────────────
      case "OPEN_HEDGE_POSITION": {
        let result: ActionResultRequest;
        try {
          const legResults = await callbacks.executeOpenPosition(nextAction.params!);
          const allSucceeded = legResults.every(lr => lr.success);
          result = {
            action: nextAction.action,
            success: allSucceeded,
            tx_hash: null,
            error: allSucceeded ? null : "One or more legs failed",
            leg_results: legResults,
          };
        } catch (err: any) {
          result = {
            action: nextAction.action,
            success: false,
            tx_hash: null,
            error: err.message ?? String(err),
            leg_results: null,
          };
        }
        await reportActionResult(intentId, result);
        await sleep(POLL_INTERVAL_FAST);
        continue;
      }

      // ─── Close Position (Safety Mode) ────────────────
      case "CLOSE_POSITION": {
        let result: ActionResultRequest;
        const protocol = nextAction.leg! as Protocol;
        try {
          const { txHash } = await callbacks.executeClosePosition(
            protocol,
            nextAction.params!
          );
          result = {
            action: nextAction.action,
            success: true,
            tx_hash: txHash,
            error: null,
            leg_results: null,
          };
        } catch (err: any) {
          result = {
            action: nextAction.action,
            success: false,
            tx_hash: null,
            error: err.message ?? String(err),
            leg_results: null,
          };
        }
        await reportActionResult(intentId, result);
        await sleep(POLL_INTERVAL_FAST);
        continue;
      }

      default:
        callbacks.onError(`Unknown action: ${nextAction.action}`);
        return;
    }
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

// ========================= Usage Example =========================

/*
// In your React component / page:

async function handleOpenHedgedPosition() {
  setLoading(true);
  
  try {
    // 1. Create the intent
    const intentId = await createHedgeIntent({
      user_id: currentUser.id,
      asset: "BTC",
      protocols: ["HL", "PACIFICA"],
      margin_usd: 1000,
      leverage: 5,
      evm_address: currentUser.evmAddress,
      solana_address: currentUser.solanaAddress,
    });
    
    // 2. Store the intentId (for resumability)
    localStorage.setItem("active_hedge_intent", intentId);
    
    // 3. Execute the saga
    await executeHedgeIntent(intentId, {
      onStatusChange: (status, detail) => {
        setCurrentStep(status);
        if (detail) setStepDetail(detail);
      },
      onError: (error) => {
        setError(error);
        setLoading(false);
      },
      onComplete: (id) => {
        setSuccess(true);
        setLoading(false);
        localStorage.removeItem("active_hedge_intent");
      },
      executeBridge: async (params, amountUsd) => {
        // Your bridge SDK integration here
        // e.g. Relay bridge
        const tx = await relaySdk.bridge({
          originChainId: params.origin_chain_id,
          destinationChainId: params.destination_chain_id,
          originCurrency: params.origin_currency,
          destinationCurrency: params.destination_currency,
          recipient: params.recipient,
          amount: usdToUsdc(amountUsd),   // convert to USDC decimals (6)
          wallet: turnkeySigner,
        });
        return { txHash: tx.hash };
      },
      executeDeposit: async (protocol, params, amountUsd) => {
        if (protocol === "HL") {
          const tx = await hyperliquidSdk.deposit({
            amount: usdToUsdc(amountUsd),
            signer: turnkeyEvmSigner,
          });
          return { txHash: tx.hash };
        } else {
          const tx = await pacificaSdk.deposit({
            amount: usdToUsdc(amountUsd),
            signer: turnkeySolanaSigner,
          });
          return { txHash: tx.signature };
        }
      },
      executeOpenPosition: async (params) => {
        const results: LegResultEntry[] = [];
        
        for (const leg of params.legs) {
          try {
            if (leg.protocol === "HL") {
              const tx = await hyperliquidSdk.openPosition({
                asset: params.asset,
                side: "LONG",  // or whichever side for HL
                margin: params.effective_margin_usd,
                leverage: params.leverage,
                signer: turnkeyEvmSigner,
              });
              results.push({
                protocol: "HL",
                success: true,
                tx_hash: tx.hash,
                error: null,
              });
            } else {
              const tx = await pacificaSdk.openPosition({
                asset: params.asset,
                side: "SHORT",  // opposite side for delta-neutrality
                margin: params.effective_margin_usd,
                leverage: params.leverage,
                signer: turnkeySolanaSigner,
              });
              results.push({
                protocol: "PACIFICA",
                success: true,
                tx_hash: tx.signature,
                error: null,
              });
            }
          } catch (err: any) {
            results.push({
              protocol: leg.protocol,
              success: false,
              tx_hash: null,
              error: err.message,
            });
          }
        }
        
        return results;
      },
      executeClosePosition: async (protocol, params) => {
        if (protocol === "HL") {
          const tx = await hyperliquidSdk.closePosition({
            asset: params.asset,
            signer: turnkeyEvmSigner,
          });
          return { txHash: tx.hash };
        } else {
          const tx = await pacificaSdk.closePosition({
            asset: params.asset,
            signer: turnkeySolanaSigner,
          });
          return { txHash: tx.signature };
        }
      },
    });
    
  } catch (err) {
    setError(`Failed to create hedge intent: ${err}`);
    setLoading(false);
  }
}

// On page load — check for in-progress intents:
useEffect(() => {
  const activeIntentId = localStorage.getItem("active_hedge_intent");
  if (activeIntentId) {
    // Resume the execution loop
    executeHedgeIntent(activeIntentId, callbacks);
  }
}, []);
*/
```

---

## 9. UI State Mapping

Map backend statuses to user-friendly UI:

```typescript
function getStepInfo(intent: HedgeIntent, legs: HedgeLeg[]): StepInfo {
  switch (intent.status) {
    case "CREATED":
    case "FUNDING": {
      const hlLeg = legs.find(l => l.protocol === "HL");
      const pacLeg = legs.find(l => l.protocol === "PACIFICA");
      
      return {
        currentStep: getFundingStep(hlLeg, pacLeg),
        progress: calculateFundingProgress(legs),
        steps: [
          { label: "Bridge to Arbitrum", status: getLegStepStatus(hlLeg, "bridge") },
          { label: "Bridge to Solana",   status: getLegStepStatus(pacLeg, "bridge") },
          { label: "Deposit to HL",      status: getLegStepStatus(hlLeg, "deposit") },
          { label: "Deposit to Pacifica",status: getLegStepStatus(pacLeg, "deposit") },
          { label: "Open Positions",     status: "pending" },
        ],
      };
    }
    case "READY":
      return {
        currentStep: "Ready to open positions",
        progress: 80,
        steps: [/* ... all funding steps complete, positions pending */],
      };
    case "OPENING":
      return {
        currentStep: "Opening hedge positions...",
        progress: 90,
        steps: [/* ... */],
      };
    case "ACTIVE":
      return {
        currentStep: "Hedge is live!",
        progress: 100,
        steps: [/* all complete */],
      };
    case "FAILED":
      const hasActiveLegs = legs.some(l => l.status === "ACTIVE" || l.status === "CLOSING");
      return {
        currentStep: hasActiveLegs
          ? "Safety mode: closing positions..."
          : "Hedge failed",
        progress: -1,
        steps: [/* ... with failure indicators */],
      };
    case "CANCELLED":
      return {
        currentStep: "Hedge cancelled & unwound",
        progress: -1,
        steps: [/* ... */],
      };
    default:
      return { currentStep: intent.status, progress: 0, steps: [] };
  }
}

// Per-leg UI status
function getLegStepStatus(
  leg: HedgeLeg | undefined,
  phase: "bridge" | "deposit"
): "pending" | "in_progress" | "done" | "error" {
  if (!leg) return "pending";
  
  if (phase === "bridge") {
    switch (leg.status) {
      case "PENDING":              return "pending";
      case "BRIDGE_IN_PROGRESS":   return "in_progress";
      case "BRIDGE_CONFIRMED":
      case "DEPOSIT_IN_PROGRESS":
      case "FUNDED":
      case "OPENING_POSITION":
      case "ACTIVE":               return "done";
      case "FAILED":               return leg.retry_count < 3 ? "in_progress" : "error";
      default:                     return "pending";
    }
  }
  
  if (phase === "deposit") {
    switch (leg.status) {
      case "PENDING":
      case "BRIDGE_IN_PROGRESS":
      case "BRIDGE_CONFIRMED":     return "pending";
      case "DEPOSIT_IN_PROGRESS":  return "in_progress";
      case "FUNDED":
      case "OPENING_POSITION":
      case "ACTIVE":               return "done";
      case "FAILED":               return leg.retry_count < 3 ? "in_progress" : "error";
      default:                     return "pending";
    }
  }
  
  return "pending";
}
```

### Suggested Progress Stepper UI

```
┌──────────────────────────────────────────────────────────────────┐
│  Open Hedged Position: BTC  |  HL ↔ Pacifica  |  $1000 × 5x    │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ✅  Bridge to Arbitrum (HL)         $500 USDC                   │
│  ⏳  Bridge to Solana (Pacifica)     $500 USDC  ← in progress   │
│  ✅  Deposit to Hyperliquid          $500 USDC                   │
│  ⬜  Deposit to Pacifica             $500 USDC                   │
│  ⬜  Open Hedge Positions            BTC 5x                      │
│                                                                  │
│  [====================............] 60%                           │
│                                                                  │
│  Status: Bridging USDC to Solana...                              │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## 10. Error Handling & Retries

### 10.1 Backend Retry Policy

- Each leg has a `retry_count` (starts at 0)
- Max retries: **3** per leg per action
- On failure: backend increments retry count & records `last_error`
- On next `next-action` call: if retries remain, same action is returned
- After 3 failures: leg → `FAILED`, intent → `FAILED`

### 10.2 FE-Side Error Handling

```typescript
// The FE should catch ALL errors and always report them.
// Never let an error silently kill the polling loop.

try {
  const { txHash } = await callbacks.executeBridge(params, amountUsd);
  // report success
} catch (err) {
  // ALWAYS report failure — don't swallow it
  await reportActionResult(intentId, {
    action: nextAction.action,
    success: false,
    tx_hash: null,
    error: err.message ?? "Unknown error",
    leg_results: null,
  });
  // The loop continues — backend will either retry or fail
}
```

### 10.3 Network Errors on `next-action` or `action-result`

If the FE can't reach the backend:
- Implement exponential backoff: 1s → 2s → 4s → 8s → 16s (max)
- Keep trying — the backend is the source of truth
- The user's intent is safely persisted

---

## 11. Edge Cases

### 11.1 User Closes Tab Mid-Bridge

- The bridge tx may still complete on-chain
- When user reopens: resume polling loop
- If bridge completed: the backend will still be waiting for a result
- **FE should check on-chain** if the bridge tx settled, then report accordingly
- If unsure: report failure → backend will retry (if retries left)

### 11.2 Both Legs Bridge, Only One Deposits

- The funded leg will be `FUNDED`, the other will be stuck
- Backend will keep returning deposit action for the stuck leg
- After 3 failures: intent → `FAILED`
- No position was opened → no safety mode needed

### 11.3 One Position Opens, Other Fails (Safety Mode)

This is the **most critical edge case**. If you report:

```json
{
  "leg_results": [
    { "protocol": "HL", "success": true, "tx_hash": "0x...", "error": null },
    { "protocol": "PACIFICA", "success": false, "tx_hash": null, "error": "slippage" }
  ]
}
```

The backend will:
1. Mark HL leg as `ACTIVE`, Pacifica leg as `FAILED`
2. Set intent to `FAILED`
3. On next `next-action` call: return `CLOSE_POSITION` for the HL leg
4. FE must close the HL position
5. After close confirmed: intent → `CANCELLED`

### 11.4 Close Position Fails in Safety Mode

- Backend retries up to 3 times
- If close permanently fails: intent stays `FAILED` with an active leg
- **This requires manual intervention** — FE should surface a clear alert

### 11.5 Duplicate Action Reports

- Backend records tx_references — if you report the same bridge success twice, it creates a duplicate tx_reference but doesn't break state
- Status transitions are idempotent (setting FUNDED when already FUNDED is fine)
- Safe to retry `action-result` calls on network failure

---

## 12. Balance-Aware Funding (Smart Skip)

The backend automatically detects pre-existing USDC balances on destination chains and within protocol margin accounts. This means **the system skips redundant bridge and deposit steps** when the user already has funds in the right places.

### 12.1 How It Works

When the first `GET /next-action` is called on a `CREATED` intent, the backend:

1. **Queries existing balances** for each leg:
   - **HL leg:** Hyperliquid clearinghouse API → `withdrawable` field (margin balance)
   - **HL on-chain:** ERC20 `balanceOf(user)` on Arbitrum USDC contract
   - **Pacifica leg:** Pacifica collateral API (margin balance)
   - **Pacifica on-chain:** Solana RPC `getTokenAccountsByOwner` for USDC

2. **Computes funding needs** per leg:
   ```
   deposit_needed = max(0, target_amount - existing_margin)
   bridge_needed  = max(0, deposit_needed - existing_onchain)
   ```

3. **Advances leg status** based on what's already available:
   - `deposit_needed == 0` → leg jumps straight to **FUNDED** (skip bridge AND deposit)
   - `bridge_needed == 0` → leg jumps to **BRIDGE_CONFIRMED** (skip bridge, still needs deposit)
   - Otherwise → leg stays **PENDING** (full bridge + deposit flow)

### 12.2 Example Scenarios

**Scenario A: User has $10 USDC in HL margin, target is $11**
```
existing_margin = $10, existing_onchain = $0
deposit_needed  = max(0, 11 - 10) = $1
bridge_needed   = max(0, 1 - 0)   = $1

→ Bridge $1 (not $11) from Base → Arb, then deposit $1
```

**Scenario B: User has $15 USDC in Pacifica margin, target is $11**
```
existing_margin = $15, existing_onchain = $0
deposit_needed  = max(0, 11 - 15) = $0

→ Leg jumps straight to FUNDED. No bridge, no deposit needed.
```

**Scenario C: User has $5 in HL margin + $8 USDC on Arbitrum, target is $11**
```
existing_margin = $5, existing_onchain = $8
deposit_needed  = max(0, 11 - 5) = $6
bridge_needed   = max(0, 6 - 8)  = $0

→ Skip bridge. Deposit $6 from on-chain balance into HL margin.
```

**Scenario D: User has $0 everywhere (fresh wallets)**
```
existing_margin = $0, existing_onchain = $0
deposit_needed  = $11
bridge_needed   = $11

→ Full flow: bridge $11, then deposit $11.
```

### 12.3 What the Frontend Sees

**Zero changes needed.** The FE already just executes whatever action the backend tells it. If the backend skips a bridge step, the FE never receives a `BRIDGE_BASE_TO_ARB` action — it goes straight to `DEPOSIT_TO_HL` or even `OPEN_HEDGE_POSITION`.

The `amount_usd` in each action reflects the **actual delta** needed, not the full target:
- Bridge action → `amount_usd` = bridge delta (e.g. $1 instead of $11)
- Deposit action → `amount_usd` = deposit delta (e.g. $6 instead of $11)

### 12.4 Balance Data in the Detail Endpoint

The `GET /hedge-intents/{id}` response now includes balance fields on each leg:

```json
{
  "legs": [
    {
      "protocol": "HL",
      "target_amount_usd": 500.0,
      "existing_margin_usd": 200.0,
      "existing_onchain_usd": 150.0,
      "funded_amount_usd": 500.0,
      "status": "FUNDED"
    }
  ]
}
```

These fields are useful for:
- **UI:** Show the user how much they already had vs. how much was newly moved
- **Debugging:** Understand why a leg skipped certain steps
- **Auditing:** Full balance snapshot at intent creation time

### 12.5 Failure Behavior

If a balance query fails (RPC down, API timeout), the backend **gracefully falls back to 0**. This means:
- A failed balance check never blocks the flow
- Worst case: user bridges/deposits the full amount (redundant but safe)
- The balance is a one-time snapshot at intent creation — not re-queried

---

## 13. Sequence Diagrams

### 12.1 Happy Path — Full Execution

```
User          Frontend                    Backend                  Chains
 │               │                           │                       │
 │  Click "Open" │                           │                       │
 │──────────────>│                           │                       │
 │               │  POST /hedge-intents      │                       │
 │               │──────────────────────────>│                       │
 │               │  { hedge_intent_id }      │                       │
 │               │<──────────────────────────│                       │
 │               │                           │                       │
 │               │  GET /{id}/next-action    │                       │
 │               │──────────────────────────>│  evaluate(CREATED)    │
 │               │  BRIDGE_BASE_TO_ARB (HL)  │  → status = FUNDING   │
 │               │<──────────────────────────│                       │
 │   Sign tx     │                           │                       │
 │<──────────────│                           │                       │
 │   ✅ signed   │                           │                       │
 │──────────────>│  Bridge Base→Arb          │                       │
 │               │───────────────────────────│──────────────────────>│
 │               │                           │            ✅ bridged │
 │               │  POST /{id}/action-result │                       │
 │               │  { success: true }        │                       │
 │               │──────────────────────────>│  leg→BRIDGE_CONFIRMED │
 │               │  { accepted }             │                       │
 │               │<──────────────────────────│                       │
 │               │                           │                       │
 │               │  GET /{id}/next-action    │                       │
 │               │──────────────────────────>│  evaluate(FUNDING)    │
 │               │  BRIDGE_BASE_TO_SOL (PAC) │                       │
 │               │<──────────────────────────│                       │
 │               │  ... (bridge + report)    │                       │
 │               │                           │                       │
 │               │  GET /{id}/next-action    │                       │
 │               │──────────────────────────>│                       │
 │               │  DEPOSIT_TO_HL            │                       │
 │               │<──────────────────────────│                       │
 │               │  ... (deposit + report)   │                       │
 │               │                           │                       │
 │               │  GET /{id}/next-action    │                       │
 │               │──────────────────────────>│                       │
 │               │  DEPOSIT_TO_PACIFICA      │                       │
 │               │<──────────────────────────│                       │
 │               │  ... (deposit + report)   │  both legs FUNDED     │
 │               │                           │  intent → READY       │
 │               │                           │                       │
 │               │  GET /{id}/next-action    │                       │
 │               │──────────────────────────>│  evaluate(READY)      │
 │               │  OPEN_HEDGE_POSITION      │  intent → OPENING     │
 │               │<──────────────────────────│                       │
 │               │                           │                       │
 │   Sign both   │                           │                       │
 │<──────────────│                           │                       │
 │   ✅ signed   │  Open Long HL             │                       │
 │──────────────>│────────────────────────────────────────────────>│
 │               │  Open Short Pacifica      │                       │
 │               │────────────────────────────────────────────────>│
 │               │                           │                       │
 │               │  POST /{id}/action-result │                       │
 │               │  { leg_results: [✅, ✅] } │                       │
 │               │──────────────────────────>│  intent → ACTIVE      │
 │               │  { accepted }             │                       │
 │               │<──────────────────────────│                       │
 │               │                           │                       │
 │               │  GET /{id}/next-action    │                       │
 │               │──────────────────────────>│  NOOP                 │
 │               │<──────────────────────────│                       │
 │               │                           │                       │
 │  "Hedge Live!"│                           │                       │
 │<──────────────│                           │                       │
```

### 12.2 Safety Mode — Partial Position Open

```
 Frontend                        Backend
   │                               │
   │  POST /{id}/action-result     │
   │  leg_results:                 │
   │    HL: ✅ success              │
   │    PAC: ❌ failed              │
   │──────────────────────────────>│
   │                               │  HL leg → ACTIVE
   │                               │  PAC leg → FAILED
   │                               │  intent → FAILED
   │  { accepted }                 │
   │<──────────────────────────────│
   │                               │
   │  GET /{id}/next-action        │
   │──────────────────────────────>│  Safety Mode!
   │  CLOSE_POSITION (HL)          │  HL leg → CLOSING
   │<──────────────────────────────│
   │                               │
   │  Close HL position...         │
   │  POST /{id}/action-result     │
   │  { success: true }            │
   │──────────────────────────────>│  HL leg → CLOSED
   │                               │  intent → CANCELLED
   │  { accepted }                 │
   │<──────────────────────────────│
   │                               │
   │  GET /{id}/next-action        │
   │──────────────────────────────>│
   │  NOOP                         │
   │<──────────────────────────────│
```

---

## Quick Reference — Chain Constants

| Chain     | Chain ID      | USDC Address                                       |
|-----------|---------------|------------------------------------------------------|
| Base      | `8453`        | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`        |
| Arbitrum  | `42161`       | `0xaf88d065e77c8cC2239327C5EDb3A432268e5831`        |
| Solana    | `792703809`   | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`    |

## Quick Reference — Protocol ↔ Chain Mapping

| Protocol    | Chain     | Signer Type |
|-------------|-----------|-------------|
| `HL`        | Arbitrum  | EVM         |
| `PACIFICA`  | Solana    | Solana      |
