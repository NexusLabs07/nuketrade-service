-- Re-entry no longer reopens the same symbol it closed: it opens whichever
-- Hyperliquid/Pacifica pair has the best 7-day spread at the time. Record which
-- one, so a close's row shows what replaced it.
ALTER TABLE pair_close_intents
    ADD COLUMN IF NOT EXISTS reentered_symbol TEXT;
