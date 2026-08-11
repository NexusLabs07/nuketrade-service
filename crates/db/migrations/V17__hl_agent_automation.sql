-- Hyperliquid agent automation: extend hl_agent_wallets with the agent's
-- lifecycle metadata, and add an audit log of every agent-signed order.
--
-- agent_name / valid_until: HL agents are approved under a name and expire
-- (HL caps expiry at 180 days; unnamed defaults are shorter). The renewal job
-- reads valid_until to re-approve before HL prunes the agent. A deregistered
-- agent address must NEVER be reused (HL prunes its nonce set, which would
-- make previously signed actions replayable), so rotation always provisions
-- a fresh Turnkey wallet.
-- da_user_id: the Turnkey user id of the delegated-access (DA) API user inside
-- this user's sub-org whose policies pin signing to this agent wallet.

ALTER TABLE hl_agent_wallets
    ADD COLUMN IF NOT EXISTS agent_name  TEXT,
    ADD COLUMN IF NOT EXISTS da_user_id  TEXT,
    ADD COLUMN IF NOT EXISTS approved_at TIMESTAMP,
    ADD COLUMN IF NOT EXISTS valid_until TIMESTAMP;

-- Every order the agent path submits (or refuses to submit) on a user's
-- behalf: manual endpoint calls and monitor-triggered closes alike. This is
-- the user-facing audit trail for "what did automation do with my account",
-- alongside Turnkey's own activity log of the signatures themselves.
CREATE TABLE IF NOT EXISTS hl_agent_actions (
    id          UUID PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action_type TEXT NOT NULL,                 -- OPEN | CLOSE
    coin        TEXT NOT NULL,
    size        TEXT,                          -- order size in coin units
    price       TEXT,                          -- slippage-bounded limit price
    reduce_only BOOLEAN NOT NULL DEFAULT FALSE,
    status      TEXT NOT NULL,                 -- SUBMITTED | FILLED | FAILED | SKIPPED
    trigger     TEXT,                          -- MANUAL | APR_BELOW_EXIT
    detail      JSONB,                         -- venue response / skip reason
    error       TEXT,
    created_at  TIMESTAMP NOT NULL DEFAULT now()
);

-- The monitor's guardrail queries (actions today, most recent action) both
-- scan a single user's recent rows.
CREATE INDEX IF NOT EXISTS idx_hl_agent_actions_user_created
    ON hl_agent_actions(user_id, created_at DESC);
