-- Pacifica agent automation: extend pacifica_agent_wallets with the delegated-
-- access user id + approval timestamp, and add an audit log of every
-- agent-signed order. Mirrors V17 (Hyperliquid).
--
-- Unlike Hyperliquid there is no agent expiry column: Pacifica agent bindings
-- persist until the master revokes them (only individual request signatures
-- expire, via expiry_window), so no renewal job is needed.
--
-- da_user_id: the Turnkey user id of the delegated-access API user inside this
-- user's sub-org whose key-pinning policy allows signing only with this agent
-- wallet. (Turnkey cannot introspect Pacifica's raw Ed25519 JSON payloads, so
-- the scope is key-pinning, not payload restriction.)

ALTER TABLE pacifica_agent_wallets
    ADD COLUMN IF NOT EXISTS da_user_id  TEXT,
    ADD COLUMN IF NOT EXISTS approved_at TIMESTAMP;

CREATE TABLE IF NOT EXISTS pacifica_agent_actions (
    id               UUID PRIMARY KEY,
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action_type      TEXT NOT NULL,             -- OPEN | CLOSE
    symbol           TEXT NOT NULL,
    amount           TEXT,                       -- order size in coin units (lot-aligned)
    slippage_percent TEXT,
    reduce_only      BOOLEAN NOT NULL DEFAULT FALSE,
    status           TEXT NOT NULL,              -- SUBMITTED | FILLED | FAILED | SKIPPED
    trigger          TEXT,                       -- MANUAL | APR_BELOW_EXIT
    client_order_id  TEXT,                       -- idempotency key sent to Pacifica
    detail           JSONB,                      -- venue response / skip reason
    error            TEXT,
    created_at       TIMESTAMP NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_pacifica_agent_actions_user_created
    ON pacifica_agent_actions(user_id, created_at DESC);
