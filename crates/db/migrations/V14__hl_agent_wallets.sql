-- Per-user Hyperliquid agent wallets.
--
-- HL's agent model is 1 agent ↔ 1 master account: the agent wallet's
-- signature alone identifies which master to route a trade to. To act on
-- N users, we therefore need N agent wallets (one per user). Each agent
-- is a Turnkey-managed wallet; the user calls HL's `approveAgent` once
-- from their main wallet to authorize it.
--
-- The automation worker reads this table per intent: if a user doesn't
-- have a row here, OR the row exists but `approved_on_hl=false`, the
-- worker fails the intent with a clear error rather than placing an
-- unsigned-by-an-unapproved-key order.

CREATE TABLE IF NOT EXISTS hl_agent_wallets (
    user_id           UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    turnkey_suborg_id TEXT NOT NULL,
    turnkey_wallet_id TEXT NOT NULL,
    evm_address       TEXT NOT NULL,
    approved_on_hl    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at        TIMESTAMP NOT NULL DEFAULT now(),
    updated_at        TIMESTAMP NOT NULL DEFAULT now()
);

-- The agent address is what HL uses to look up the master. Each agent
-- address must map to a single user — duplicates would break routing.
CREATE UNIQUE INDEX IF NOT EXISTS uq_hl_agent_wallets_evm_address
    ON hl_agent_wallets(evm_address);
