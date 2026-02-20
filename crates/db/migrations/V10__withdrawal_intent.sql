-- Withdrawal Intents: tracks a withdraw-from-protocol + bridge-to-Base flow
-- Status lifecycle: CREATED → WITHDRAWING → WITHDRAWN → BRIDGING → COMPLETED | FAILED
CREATE TABLE IF NOT EXISTS withdrawal_intents (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    exchange VARCHAR(20) NOT NULL,          -- 'hyperliquid' | 'pacifica' | 'lighter'
    amount_usd DOUBLE PRECISION NOT NULL,
    evm_address VARCHAR(42) NOT NULL,
    recipient VARCHAR(42) NOT NULL,         -- destination address on Base
    destination_chain_id INT NOT NULL,      -- Base = 8453
    status VARCHAR(30) NOT NULL DEFAULT 'CREATED',
    retry_count SMALLINT NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

-- Withdrawal Steps: immutable log of each on-chain action for the intent
-- step: 'WITHDRAW' (protocol → source chain) | 'BRIDGE' (source chain → Base)
-- Status lifecycle: PENDING → SUBMITTED → CONFIRMED | FAILED
CREATE TABLE IF NOT EXISTS withdrawal_steps (
    id UUID PRIMARY KEY,
    withdrawal_intent_id UUID NOT NULL REFERENCES withdrawal_intents(id) ON DELETE CASCADE,
    step VARCHAR(20) NOT NULL,
    tx_hash VARCHAR(128),
    chain_id INT,
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE INDEX idx_withdrawal_intents_user_id ON withdrawal_intents(user_id);
CREATE INDEX idx_withdrawal_intents_status ON withdrawal_intents(status);
CREATE INDEX idx_withdrawal_steps_intent_id ON withdrawal_steps(withdrawal_intent_id);
