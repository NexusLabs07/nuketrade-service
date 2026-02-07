-- Hedge Intents: the atomic unit of a hedged position request
CREATE TABLE IF NOT EXISTS hedge_intents (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    asset VARCHAR(20) NOT NULL,
    protocol_a VARCHAR(20) NOT NULL,
    protocol_b VARCHAR(20) NOT NULL,
    margin_usd DOUBLE PRECISION NOT NULL,
    leverage DOUBLE PRECISION NOT NULL,
    evm_address VARCHAR(42) NOT NULL,
    solana_address VARCHAR(44) NOT NULL,
    status VARCHAR(30) NOT NULL DEFAULT 'CREATED',
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

-- Hedge Legs: one per protocol per intent (always exactly 2)
CREATE TABLE IF NOT EXISTS hedge_legs (
    id UUID PRIMARY KEY,
    hedge_intent_id UUID NOT NULL REFERENCES hedge_intents(id) ON DELETE CASCADE,
    protocol VARCHAR(20) NOT NULL,
    chain VARCHAR(20) NOT NULL,
    target_amount_usd DOUBLE PRECISION NOT NULL,
    funded_amount_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
    status VARCHAR(30) NOT NULL DEFAULT 'PENDING',
    retry_count SMALLINT NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

-- Transaction References: immutable log of all on-chain actions
CREATE TABLE IF NOT EXISTS tx_references (
    id UUID PRIMARY KEY,
    hedge_leg_id UUID NOT NULL REFERENCES hedge_legs(id) ON DELETE CASCADE,
    action VARCHAR(50) NOT NULL,
    tx_hash VARCHAR(128),
    chain VARCHAR(20) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    created_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE INDEX idx_hedge_intents_user_id ON hedge_intents(user_id);
CREATE INDEX idx_hedge_intents_status ON hedge_intents(status);
CREATE INDEX idx_hedge_legs_intent_id ON hedge_legs(hedge_intent_id);
CREATE INDEX idx_hedge_legs_status ON hedge_legs(status);
CREATE INDEX idx_tx_references_leg_id ON tx_references(hedge_leg_id);
