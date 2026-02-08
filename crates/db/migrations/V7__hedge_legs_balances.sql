-- Track pre-existing balances per leg so the backend can skip redundant bridges/deposits.
-- existing_margin_usd:  USDC already inside the protocol's margin account
-- existing_onchain_usd: USDC sitting on the destination chain (not yet deposited)
ALTER TABLE hedge_legs ADD COLUMN existing_margin_usd DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE hedge_legs ADD COLUMN existing_onchain_usd DOUBLE PRECISION NOT NULL DEFAULT 0;
