-- Durable actual-fill evidence for the normal user-triggered hedge saga.
--
-- The second leg is sized from the first venue's observed fill, and safety
-- compensation closes the persisted filled amount rather than the requested
-- funding amount.

ALTER TABLE hedge_legs
    ADD COLUMN filled_amount_usd DOUBLE PRECISION;

ALTER TABLE hedge_legs
    ADD COLUMN venue_order_id TEXT;

ALTER TABLE hedge_legs
    ADD COLUMN average_fill_price DOUBLE PRECISION;

ALTER TABLE hedge_legs
    ADD COLUMN funding_intent_id UUID;
