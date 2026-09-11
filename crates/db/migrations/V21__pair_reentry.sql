-- Re-entry after an APR exit. When the pair-exit monitor closes a hedge because
-- its funding spread fell below the user's exit threshold, the close now records
-- enough to reopen that same hedge (notional + leverage of the HL leg; symbol
-- and sides were already stored), and a separate re-entry monitor reopens it
-- once the spread recovers above the user's min_apr_to_enter.
--
-- Only APR-triggered closes are candidates: a MANUAL close is the user's own
-- decision, not a pause, and is never reopened. Per-close re-entry status:
--   NONE        not a candidate (manual close, or a row from before this feature)
--   PENDING     eligible: reopen once the spread APR recovers
--   REOPENED    the re-entry monitor reopened the pair
--   SUPERSEDED  a newer APR close for the same user+symbol replaced this one
--   ABANDONED   gave up: a position was found already open, or too many failures
-- The DEFAULT is NONE so every existing row is left alone.

ALTER TABLE pair_close_intents
    ADD COLUMN IF NOT EXISTS notional_per_leg_usd    DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS leverage                DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS reentry_status          TEXT NOT NULL DEFAULT 'NONE',
    ADD COLUMN IF NOT EXISTS reentry_attempts        INT  NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS reentry_last_error      TEXT,
    ADD COLUMN IF NOT EXISTS reentry_last_attempt_at TIMESTAMP,
    ADD COLUMN IF NOT EXISTS reentered_at            TIMESTAMP;

-- The re-entry monitor's per-user scan: "PENDING closes for this user".
CREATE INDEX IF NOT EXISTS idx_pair_close_intents_reentry
    ON pair_close_intents(reentry_status, user_id, symbol);
