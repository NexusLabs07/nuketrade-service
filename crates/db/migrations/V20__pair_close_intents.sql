-- Coordinated two-leg close sagas. A delta-neutral position is a PAIR (same
-- symbol, opposite sides on Hyperliquid + Pacifica); exiting it means closing
-- BOTH legs together — closing one alone leaves naked directional exposure.
-- One row per pair-close attempt.
--
-- Status machine:
--   CLOSING           both legs being closed (fresh, or both failed and retrying)
--   LEG_OPEN_HL       Pacifica leg closed, HL leg still open  <- NAKED, retried
--   LEG_OPEN_PACIFICA HL leg closed, Pacifica leg still open  <- NAKED, retried
--   COMPLETED         both legs flat
--   FAILED            both legs failed repeatedly; pair is still fully hedged
--                     (safe), recorded for visibility
-- The LEG_OPEN_* states are retried on every monitor tick with no retry cap:
-- we keep trying to unwind so we never rest in the naked state (same policy as
-- the hedge state machine's safety mode).

CREATE TABLE IF NOT EXISTS pair_close_intents (
    id             UUID PRIMARY KEY,
    user_id        UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    symbol         TEXT NOT NULL,
    hl_side        TEXT NOT NULL,              -- Long | Short (HL leg at decision time)
    pacifica_side  TEXT NOT NULL,
    spread_apr_pct DOUBLE PRECISION,           -- pair funding-spread APR at decision
    status         TEXT NOT NULL,
    trigger        TEXT,                       -- MANUAL | APR_BELOW_EXIT
    retry_count    INT NOT NULL DEFAULT 0,
    last_error     TEXT,
    created_at     TIMESTAMP NOT NULL DEFAULT now(),
    updated_at     TIMESTAMP NOT NULL DEFAULT now()
);

-- The monitor scans for in-flight sagas (resume partial closes first) and
-- checks "is there already an active intent for this user+symbol".
CREATE INDEX IF NOT EXISTS idx_pair_close_intents_status
    ON pair_close_intents(status);
CREATE INDEX IF NOT EXISTS idx_pair_close_intents_user_symbol
    ON pair_close_intents(user_id, symbol, status);
