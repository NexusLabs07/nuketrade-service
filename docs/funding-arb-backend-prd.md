# PRD — Funding-Rate Arbitrage Backend Agent

**Status:** Draft for implementation
**Audience:** Backend coding agent
**Goal:** Build a service that automatically opens and closes perpetual-futures positions on behalf of users to harvest funding-rate yield, signing every trade through Turnkey under tightly scoped policies.

---

## 0. How to use this document

This PRD is written to be implemented directly. Each component has **Responsibilities**, a **Spec**, and **Acceptance criteria**. Where a product/architecture choice is still open, it is marked **[DECISION]** and a default is given so you are never blocked — but confirm these with the product owner before shipping to production. Pseudocode is illustrative, not prescriptive about language idioms.

---

## 1. Overview

### 1.1 What it does
Users opt in and allocate capital. The system continuously watches funding rates across supported perpetual markets. For each user, when a market's annualized funding rate (APY) rises above that user's **open threshold**, the system opens a position to collect funding. When the APY falls below that user's **close threshold**, it closes the position (and capital is freed to rotate into a better market).

### 1.2 Core design principle
The per-tick decision path must never touch the database. Funding rates arrive ~1/second over WebSocket; the matching logic runs against an **in-memory threshold ladder** and is **edge-triggered** (acts only when a threshold is crossed, not every tick). The DB is the durable source of truth, synced into memory via write-through. Execution is **decoupled** onto a job queue so the detection loop never blocks on signing or network I/O.

### 1.3 Out of scope (v1)
- A user-facing frontend (this is backend only).
- The perpetual DEX matching engine itself — we *trade on* a perp venue, we don't build one. (See **[DECISION] D1**.)
- Fiat on/off ramps.
- Tax/accounting export.

---

## 2. Glossary

- **Funding rate** — periodic payment between longs and shorts on a perp to keep its price near spot. Sampled frequently; settled on the venue's funding interval.
- **APY** — the funding rate annualized to a comparable yearly percentage. All thresholds are expressed against this.
- **bps** — basis points (1% = 100 bps). All rates and thresholds are stored as integer bps to avoid floating-point drift.
- **Open / close threshold** — per-user APY levels that trigger opening or closing.
- **Edge-triggered** — fire once on the transition across a level, not repeatedly while above/below it.
- **Ladder** — in-memory sorted structure mapping threshold levels to the set of users at that level.
- **Delegated access** — a Turnkey pattern where an agent key may sign on a user's behalf, constrained by policy.

---

## 3. Decisions to confirm (do not skip)

- **[DECISION] D1 — Perp venue(s).** The venue is abstracted behind a `VenueAdapter` interface (§7). Default assumption: a single external perp DEX accessed via REST/WS + on-chain contract calls. *Confirm which venue(s) so the first adapter can be written.*
- **[DECISION] D2 — Custody model.**
  - *Default (recommended):* **per-user Turnkey sub-organization** — each user has their own wallet; the agent key gets scoped delegated access per sub-org. Non-custodial; blast radius is one user.
  - *Alternative:* **pooled wallet** — one Turnkey wallet holds all collateral; DB tracks each user's share. Simpler but custodial. *Confirm.*
- **[DECISION] D3 — Strategy mode.** A naked single-leg perp position collects funding but carries full price risk; it is **not** true arbitrage. Options:
  - `SINGLE_LEG` (default for v1, simplest) — open one perp leg in the funding-favorable direction.
  - `DELTA_NEUTRAL` — open the perp leg plus an offsetting hedge (spot or opposing venue) so price risk nets out. More complex; flag if required for v1.
- **[DECISION] D4 — Capital contention.** When multiple markets are simultaneously eligible for one user, how is capital allocated? Default: rank by smoothed APY, fill highest first up to `capital_allocated`.
- **[DECISION] D5 — Funding interval & smoothing window** per venue. Default smoothing: EMA over a 60-sample (~60s) window; require the smoothed value to stay across the threshold to trigger.

---

## 4. System architecture

Components and the two independent clocks:

1. **Market Data Ingestion** — WS client(s) per venue; computes and smooths APY per market.
2. **Matching Engine (hot path)** — in-memory ladder; edge-triggered crossing detection; emits intents.
3. **Config Store** — DB-backed user/strategy config + in-memory mirror with write-through sync.
4. **Execution Service** — job queue + workers; builds txs, signs via Turnkey, broadcasts, retries.
5. **Turnkey Integration** — sub-org/wallet provisioning, policy provisioning, signing client.
6. **Risk Manager (independent fast loop)** — monitors collateral/liquidation; owns the kill switch.
7. **API Layer** — user config CRUD, deposit/withdraw intents, position/status reads.
8. **Persistence** — Postgres (source of truth) + a queue (e.g., Redis/NATS/SQS).
9. **Observability** — structured logs, metrics, full audit log of signed actions.

**Hot path** = WS tick → compute APY → smooth → ladder crossing check → (on cross) enqueue. Nothing else runs per tick.
**Cold/async path** = workers consume jobs → build tx → Turnkey sign → broadcast → confirm → write DB.
**Risk loop** = separate clock (~every few seconds), reads positions, can trip the kill switch.

---

## 5. Data model (Postgres)

Use integer bps for all rate fields. Use a numeric/decimal type for token amounts (never float).

```sql
-- One row per onboarded user
users (
  id              UUID PRIMARY KEY,
  turnkey_sub_org_id  TEXT NOT NULL,      -- D2: present in per-user model
  wallet_address  TEXT NOT NULL,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Per-user strategy settings (the thresholds live here durably)
user_strategy_config (
  user_id            UUID PRIMARY KEY REFERENCES users(id),
  enabled            BOOLEAN NOT NULL DEFAULT false,
  open_threshold_bps  INTEGER NOT NULL,   -- e.g. 1000 = 10% APY
  close_threshold_bps INTEGER NOT NULL,   -- e.g. 200  = 2%  APY
  allowed_markets    TEXT[]  NOT NULL,    -- market symbols this user permits
  max_position_size  NUMERIC NOT NULL,    -- per-position cap (token units)
  capital_allocated  NUMERIC NOT NULL,    -- total capital this user commits
  mode               TEXT NOT NULL DEFAULT 'SINGLE_LEG', -- D3
  updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- INVARIANT: open_threshold_bps > close_threshold_bps (hysteresis gap)

-- Position lifecycle
positions (
  id                 UUID PRIMARY KEY,
  user_id            UUID NOT NULL REFERENCES users(id),
  market             TEXT NOT NULL,
  side               TEXT NOT NULL,       -- LONG | SHORT
  size               NUMERIC NOT NULL,
  entry_price        NUMERIC,
  status             TEXT NOT NULL,       -- PENDING|OPEN|CLOSING|CLOSED|FAILED
  turnkey_activity_id TEXT,               -- signing activity reference
  tx_hash            TEXT,
  opened_at          TIMESTAMPTZ,
  closed_at          TIMESTAMPTZ
);
-- INDEX on (user_id, market, status) to look up "in position?" quickly at startup load.

-- Async execution jobs (idempotent)
execution_jobs (
  id            UUID PRIMARY KEY,
  dedupe_key    TEXT UNIQUE NOT NULL,     -- see §8.3
  user_id       UUID NOT NULL,
  intent        TEXT NOT NULL,            -- OPEN | CLOSE
  market        TEXT NOT NULL,
  status        TEXT NOT NULL,            -- QUEUED|PROCESSING|DONE|FAILED
  attempts      INTEGER NOT NULL DEFAULT 0,
  last_error    TEXT,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Append-only audit of every signing action
audit_log (
  id            UUID PRIMARY KEY,
  user_id       UUID,
  action        TEXT NOT NULL,            -- OPEN|CLOSE|KILL_SWITCH|POLICY_UPDATE
  turnkey_activity_id TEXT,
  tx_hash       TEXT,
  payload       JSONB,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Global protocol defaults (NOT duplicated per user)
protocol_config (
  key   TEXT PRIMARY KEY,
  value JSONB NOT NULL
);
```

**Acceptance criteria**
- All rate fields are integer bps; all token amounts are decimal/numeric, never float.
- DB enforces `open_threshold_bps > close_threshold_bps`.
- Reading "is user U in a position on market M" is a single indexed lookup.

---

## 6. Market Data Ingestion

**Responsibilities:** maintain WS connections per venue, normalize funding rates to APY, smooth them, hand each smoothed sample to the Matching Engine.

**Spec**
- One WS subscription per venue per supported market (or a multiplexed stream where the venue supports it).
- On each raw funding sample: `apy_bps = annualize(raw_rate, venue.funding_interval)`. Annualization formula is venue-specific; encapsulate in the `VenueAdapter`.
- Smoothing: maintain per-market EMA (or rolling window) per **[DECISION] D5**. Emit the smoothed value, not the raw tick.
- Reconnect with backoff on WS drop; on reconnect, do **not** replay stale samples as new crossings (reset `last_apy` baseline cleanly).
- Backpressure: if a tick arrives while the previous is still processing, drop to the latest (last-value-wins); never queue raw ticks unbounded.

**Acceptance criteria**
- A market with no rate change for an hour produces near-zero matching-engine work.
- WS disconnect/reconnect does not generate spurious open/close events.

---

## 7. Venue Adapter interface  **[DECISION] D1**

A clean boundary so additional venues plug in without touching the engine.

```
interface VenueAdapter {
  subscribeFunding(markets): emits { market, rawRate, ts }
  annualize(rawRate): apy_bps
  buildOpenTx(user, market, side, size): UnsignedTx
  buildCloseTx(position): UnsignedTx
  getMarginInfo(position): { collateral, markPrice, liquidationPrice, marginRatio }
  routerAddress: string                 // the only contract the agent may call
  functionSelectors: { open, close }    // allowlisted selectors for Turnkey policy
}
```

**Acceptance criteria**
- The engine, execution, and risk components reference only this interface, never venue specifics.

---

## 8. Matching Engine (hot path) — the core

**Responsibilities:** detect threshold crossings per market and emit OPEN/CLOSE intents, entirely in memory, edge-triggered.

### 8.1 In-memory structure (per market)
```
MarketLadder {
  open_rungs:  SortedMap<threshold_bps, Set<user_id>>   // users who want to OPEN at/above level
  close_rungs: SortedMap<threshold_bps, Set<user_id>>   // users who want to CLOSE below level
  last_apy_bps: int
}
```
Bucket users by threshold value — there are far fewer distinct levels than users.

### 8.2 Crossing detection (edge-triggered)
```
on_smoothed_apy(market, apy_bps):
    ladder = ladders[market]
    prev = ladder.last_apy_bps

    if apy_bps > prev:                                   # upward crossing -> opens
        for (level, users) in ladder.open_rungs.range(prev exclusive .. apy_bps inclusive):
            for u in users:
                if not in_position(u, market) and not in_flight(u, market):
                    enqueue(OPEN, u, market)

    elif apy_bps < prev:                                 # downward crossing -> closes
        for (level, users) in ladder.close_rungs.range(apy_bps inclusive .. prev exclusive):
            for u in users:
                if in_position(u, market) and not in_flight(u, market):
                    enqueue(CLOSE, u, market)

    ladder.last_apy_bps = apy_bps
```
- `in_position` / `in_flight` are answered from in-memory state, not the DB.
- Boundary semantics: open when `apy >= open_threshold`; close when `apy < close_threshold`. Implement the range bounds to match exactly.

### 8.3 Idempotency / in-flight guard
- `dedupe_key = hash(user_id, market, intent, current_funding_epoch)`. Enqueue is a no-op if the key already exists.
- Mark `in_flight(u, market)=true` on enqueue; clear it when the job reaches a terminal state.

**Acceptance criteria**
- Processing one tick across all markets is O(crossed rungs), not O(users). Target **< 5 ms** per tick at 10k users / 50 markets.
- No DB read or write occurs on the tick path.
- A rate oscillating between the open and close levels does not produce repeated trades (verified by the hysteresis gap + edge-triggering).
- A second crossing while a prior job is in flight does not double-open (verified by dedupe + in-flight guard).

---

## 9. Config Store (DB + in-memory mirror)

**Responsibilities:** hold durable config; serve the engine's in-memory ladders; keep them in sync.

**Spec**
- On startup: load all `enabled` users into the ladders and load open-position state into the in-memory `in_position` index.
- Write-through: every config mutation (§11 API) updates Postgres **and** the in-memory ladders atomically (update DB, then apply the same delta to memory; on failure, do not partially apply).
- Adding/removing a user at a threshold = set insert/delete on the relevant rung.

**Acceptance criteria**
- A threshold change is reflected in matching within one tick, with no full reload.
- A cold restart reconstructs identical in-memory state from the DB.

---

## 10. Execution Service (async)

**Responsibilities:** consume intents, perform the trade safely, persist results.

**Spec**
- Worker pool consumes the queue. Per job:
  1. Re-validate preconditions against current state (still eligible? still in/out of position? within caps? not killed?).
  2. Ask `VenueAdapter` to build the unsigned tx.
  3. Request signature from Turnkey (§11/Turnkey integration).
  4. Broadcast; await confirmation.
  5. Update `positions`, clear `in_flight`, write `audit_log`.
- Retries: bounded with backoff. Distinguish retryable (RPC timeout) from terminal (policy-denied, insufficient margin) failures.
- On terminal failure: mark `positions.status=FAILED`, clear in-flight, emit alert.
- All jobs idempotent (re-running a confirmed job is a no-op via dedupe + on-chain state check).

**Acceptance criteria**
- A worker crash mid-job does not lose or duplicate a trade (at-least-once queue + idempotency).
- A Turnkey policy denial is logged, surfaced, and never silently retried forever.

---

## 11. Turnkey integration  **[DECISION] D2]**

**Responsibilities:** provision per-user wallets, provision and maintain signing policies, sign transactions.

**Spec (per-user sub-org default)**
- **Onboarding:** create a Turnkey sub-organization per user containing their wallet; record `turnkey_sub_org_id` + `wallet_address`.
- **Agent key:** a service-controlled API key with **delegated access** to sign on each user's behalf. The private key never leaves Turnkey; the backend only requests signatures.
- **Policy (principle of least privilege):** the agent may sign a transaction **only if all** hold:
  - destination == `VenueAdapter.routerAddress` (allowlisted contract),
  - function selector ∈ `{open, close}` (allowlisted selectors),
  - size / value ≤ per-user cap.
  - Everything else (withdrawals, transfers to arbitrary addresses, other contracts) is denied by default.
- **DENY overrides ALLOW** — use a DENY policy as the circuit breaker.
- **Kill switch:** risk manager can lock the wallet / activate a DENY policy to immediately block all signing, overriding existing allows.
- **Signing client:** submit the unsigned tx as a Turnkey signing activity, retrieve the signature, return it to the execution worker. Record `turnkey_activity_id`.

> Implementation note: use the current Turnkey SDK and consult live docs for exact activity/policy API shapes (https://docs.turnkey.com) — method names and policy schema may differ from any cached knowledge. Treat the behavior above as the contract.

**Acceptance criteria**
- With a correct policy in place, an attempt to sign a transfer to an arbitrary address is rejected by Turnkey, not by app code.
- Turnkey API credentials are stored in a secret manager, never in the DB or repo.
- Every successful signature has a corresponding `audit_log` row with its `turnkey_activity_id`.

---

## 12. Risk Manager (independent loop)

**Responsibilities:** protect capital regardless of funding logic.

**Spec**
- Runs on its own clock (~every few seconds), independent of funding cadence.
- For each open position: fetch `getMarginInfo`; compute distance to liquidation.
- If margin ratio < configured floor: trigger emergency close (high-priority job) and/or trip the kill switch.
- Kill switch: sets a global `paused` flag (execution workers stop dequeuing) **and** activates the Turnkey DENY policy / wallet lock.
- Sanity checks: reject obviously bad oracle/mark prices (e.g., zero, stale, > X% jump) before acting.

**Acceptance criteria**
- Liquidation-risk detection does not depend on the funding tick loop running.
- Tripping the kill switch halts new signing within one risk-loop interval.

---

## 13. API Layer

REST (or gRPC) endpoints, authenticated. No endpoint triggers trades directly — they mutate config/state; the engine reacts.

- `POST /users` — onboard (provision sub-org/wallet).
- `GET /users/{id}/config` / `PUT /users/{id}/config` — read/update thresholds, allowed markets, caps, enabled, mode. (Write-through to ladders.)
- `POST /users/{id}/deposit` / `POST /users/{id}/withdraw` — capital intents.
- `GET /users/{id}/positions` — current positions + status.
- `GET /health`, `GET /metrics`.
- `POST /admin/kill-switch` — manual global pause (admin-only).

**Acceptance criteria**
- Updating `enabled=false` removes the user from matching within one tick and does not affect existing open positions' close logic unless specified.
- Invalid config (e.g., open ≤ close) is rejected with a 4xx before any write.

---

## 14. Non-functional requirements

- **Performance:** matching tick budget < 5 ms (§8); DB never in the hot path.
- **Correctness:** all money math in decimal; all rates in integer bps.
- **Reliability:** at-least-once queue + idempotent jobs; crash-safe state reconstruction on restart.
- **Security:** least-privilege Turnkey policies; secrets in a vault; full audit trail; admin endpoints authz-gated.
- **Observability:** structured logs, metrics (ticks/s, crossings/s, jobs by status, signing latency, kill-switch state), alerting on terminal failures and kill-switch trips.

---

## 15. Suggested tech stack (non-binding)

- Language: TypeScript/Node (aligns with Turnkey SDK and WS handling) or Go. The hot-path matching module can be isolated and rewritten in a faster runtime later if needed — at ~1 sample/sec it is not a bottleneck.
- DB: Postgres. Queue: Redis Streams / NATS / SQS. Cache/mirror: in-process memory.
- Keep the `VenueAdapter` and Turnkey client as swappable modules.

---

## 16. Suggested build order (milestones)

1. **M1 — Skeleton + persistence:** schema, config CRUD API, write-through to an in-memory store (no trading yet).
2. **M2 — Market data + matching:** venue WS adapter (one venue), APY + smoothing, in-memory ladder, edge-triggered crossing detection emitting *logged* intents (dry run, no signing).
3. **M3 — Turnkey provisioning + policy:** sub-org/wallet creation, least-privilege policy, signing client; verify a denied action is blocked by policy.
4. **M4 — Execution service:** queue, workers, build/sign/broadcast/confirm, idempotency, retries, position persistence.
5. **M5 — Risk manager + kill switch.**
6. **M6 — Observability, hardening, restart-safety tests, end-to-end on testnet.**

Each milestone ships with the acceptance criteria from its referenced section as tests.

---

## 17. Test scenarios (must pass)

- Rate oscillating around the open level produces exactly one open, not a stream.
- Rate crossing up then immediately down within one tick interval resolves to a single consistent state.
- Worker killed mid-trade → on restart, no duplicate position, no orphaned in-flight flag.
- Policy denies a forged withdrawal transaction at the Turnkey layer.
- Kill switch halts signing within one risk interval and blocks queued jobs.
- 10k users / 50 markets sustained at 1 tick/sec/market within the 5 ms tick budget.
