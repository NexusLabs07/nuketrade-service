-- Cross-venue rebalance sagas (Phase 3). When a delta-neutral hedge's two legs
-- drift apart in equity (price moved), collateral is moved from the rich leg to
-- the poor one: withdraw (master-key, destination-pinned) -> bridge -> deposit.
-- This table tracks the multi-step money path so a saga can be resumed/audited.
--
-- Only the withdraw leg is executed by the backend today; the bridge/deposit
-- legs need their own DA transaction-signing policies and are tracked as the
-- next increment (status stops at AWAITING_TRANSFER after the withdrawal).

CREATE TABLE IF NOT EXISTS rebalance_intents (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    from_exchange   TEXT NOT NULL,             -- rich leg (withdraw source)
    to_exchange     TEXT NOT NULL,             -- poor leg (deposit target)
    amount_usd      DOUBLE PRECISION NOT NULL,
    status          TEXT NOT NULL,             -- DETECTED | WITHDRAWING | WITHDRAWN
                                               -- | AWAITING_TRANSFER | BRIDGING | BRIDGED
                                               -- | DEPOSITING | COMPLETED | FAILED
    from_equity_usd DOUBLE PRECISION,
    to_equity_usd   DOUBLE PRECISION,
    withdraw_tx_ref TEXT,
    bridge_tx_ref   TEXT,
    deposit_tx_ref  TEXT,
    trigger         TEXT,                       -- MANUAL | IMBALANCE
    last_error      TEXT,
    created_at      TIMESTAMP NOT NULL DEFAULT now(),
    updated_at      TIMESTAMP NOT NULL DEFAULT now()
);

-- The monitor's "is a rebalance already in flight for this user" guard scans a
-- user's recent rows by status.
CREATE INDEX IF NOT EXISTS idx_rebalance_intents_user_status
    ON rebalance_intents(user_id, status);
