-- Durable actual-fill evidence for the normal user-triggered hedge saga.
-- The second leg is sized from the first venue's observed fill, and safety-mode
-- compensation closes the persisted filled amount rather than the requested
-- funding amount.

ALTER TABLE hedge_legs
    ADD COLUMN IF NOT EXISTS filled_amount_usd DOUBLE PRECISION;

ALTER TABLE hedge_legs
    ADD COLUMN IF NOT EXISTS venue_order_id TEXT;

ALTER TABLE hedge_legs
    ADD COLUMN IF NOT EXISTS average_fill_price DOUBLE PRECISION;

ALTER TABLE hedge_legs
    ADD COLUMN IF NOT EXISTS funding_intent_id UUID;
