-- Durable Solana-native Bulk withdrawal intents.
--
-- Opcode 4 is a signal-only user transaction. Bulk validators later perform
-- the vault settlement, so the exact request transaction and destination are
-- retained until account/activity and recipient-chain evidence agree.

CREATE TABLE IF NOT EXISTS bulk_withdrawal_intents (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    network VARCHAR(16) NOT NULL CHECK (network IN ('mainnet', 'testnet')),
    account TEXT NOT NULL,
    signer TEXT NOT NULL,
    fee_payer TEXT NOT NULL,
    mint TEXT NOT NULL,
    program_id TEXT NOT NULL,
    recipient_wallet TEXT NOT NULL,
    recipient_token_account TEXT NOT NULL,
    vault TEXT NOT NULL,
    vault_token_account TEXT NOT NULL,
    amount_base_units TEXT NOT NULL CHECK (amount_base_units ~ '^[1-9][0-9]*$'),
    recipient_balance_before_base_units TEXT NOT NULL
        CHECK (recipient_balance_before_base_units ~ '^[0-9]+$'),
    idempotency_key TEXT NOT NULL CHECK (length(btrim(idempotency_key)) BETWEEN 1 AND 255),
    idempotency_fingerprint CHAR(64) NOT NULL CHECK (idempotency_fingerprint ~ '^[0-9a-f]{64}$'),
    recent_blockhash TEXT NOT NULL,
    last_valid_block_height BIGINT,
    message_base64 TEXT NOT NULL,
    signed_transaction_base64 TEXT,
    solana_signature TEXT,
    solana_slot BIGINT,
    settlement_activity_slot BIGINT,
    status VARCHAR(32) NOT NULL CHECK (
        status IN (
            'PREPARED',
            'SUBMITTING',
            'SOLANA_FINALIZED',
            'SETTLEMENT_PENDING',
            'COMPLETED',
            'REJECTED',
            'UNKNOWN'
        )
    ),
    CONSTRAINT bulk_withdrawal_signature_pair_check CHECK (
        (signed_transaction_base64 IS NULL) = (solana_signature IS NULL)
    ),
    CONSTRAINT bulk_withdrawal_status_identity_check CHECK (
        (
            status = 'PREPARED'
            AND signed_transaction_base64 IS NULL
            AND solana_signature IS NULL
        )
        OR (
            status IN (
                'SUBMITTING',
                'SOLANA_FINALIZED',
                'SETTLEMENT_PENDING',
                'COMPLETED',
                'UNKNOWN'
            )
            AND signed_transaction_base64 IS NOT NULL
            AND solana_signature IS NOT NULL
        )
        OR status = 'REJECTED'
    ),
    raw_response_metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_error TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now(),
    CONSTRAINT uq_bulk_withdrawal_intents_idempotency
        UNIQUE (user_id, network, idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_bulk_withdrawal_intents_status_updated
    ON bulk_withdrawal_intents (network, status, updated_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS uq_bulk_withdrawal_intents_solana_signature
    ON bulk_withdrawal_intents (network, solana_signature)
    WHERE solana_signature IS NOT NULL;

-- A withdrawal activity row does not contain the originating Solana request
-- signature. Serialize active withdrawals per user/network so reconciliation
-- cannot attribute one user's equal-sized request to another request.
CREATE UNIQUE INDEX IF NOT EXISTS uq_bulk_withdrawal_intents_one_active_user
    ON bulk_withdrawal_intents (user_id, network)
    WHERE status IN (
        'PREPARED',
        'SUBMITTING',
        'SOLANA_FINALIZED',
        'SETTLEMENT_PENDING',
        'UNKNOWN'
    );
