-- Consolidated Bulk persistence schema.
--
-- This migration is intentionally the first production Bulk migration. It
-- contains the final schema for signed-request recovery, agent authorization,
-- native deposits, and native withdrawals without carrying forward the
-- intermediate ALTER TABLE steps used during feature development.

-- Identity-scoped nonce allocation prevents a restart, clock rollback, or
-- concurrent request from reusing a Bulk signed-request nonce.
CREATE TABLE bulk_nonce_state (
    account TEXT NOT NULL,
    signer TEXT NOT NULL,
    network VARCHAR(16) NOT NULL CHECK (network IN ('mainnet', 'testnet')),
    last_nonce BIGINT NOT NULL CHECK (last_nonce >= 0),
    updated_at TIMESTAMP NOT NULL DEFAULT now(),
    PRIMARY KEY (account, signer, network)
);

-- Append-oriented journal for every signed Bulk mutation. Idempotency fields
-- are nullable because reconciliation and internal recovery records may not
-- originate from an HTTP request carrying an Idempotency-Key.
CREATE TABLE bulk_execution_journal (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    account TEXT NOT NULL,
    signer TEXT NOT NULL,
    network VARCHAR(16) NOT NULL CHECK (network IN ('mainnet', 'testnet')),
    nonce BIGINT NOT NULL CHECK (nonce >= 0),
    canonical_request_hash CHAR(64) NOT NULL,
    computed_bulk_order_id TEXT,
    idempotency_key TEXT,
    idempotency_fingerprint TEXT,
    action_type VARCHAR(64) NOT NULL,
    symbol TEXT,
    requested_size DOUBLE PRECISION,
    reduce_only BOOLEAN,
    status VARCHAR(32) NOT NULL CHECK (
        status IN (
            'PREPARED',
            'SUBMITTING',
            'ACCEPTED',
            'REJECTED',
            'UNKNOWN',
            'RECONCILED_ACCEPTED',
            'RECONCILED_REJECTED'
        )
    ),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    raw_request_metadata JSONB NOT NULL,
    raw_response_metadata JSONB,
    last_error TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE (account, signer, network, nonce),
    CONSTRAINT bulk_execution_journal_idempotency_pair_check CHECK (
        (idempotency_key IS NULL AND idempotency_fingerprint IS NULL)
        OR (
            idempotency_key IS NOT NULL
            AND idempotency_fingerprint IS NOT NULL
            AND length(btrim(idempotency_key)) BETWEEN 1 AND 255
            AND length(idempotency_fingerprint) = 64
        )
    )
);

CREATE INDEX idx_bulk_execution_journal_user_created
    ON bulk_execution_journal (user_id, created_at DESC);

CREATE INDEX idx_bulk_execution_journal_unknown
    ON bulk_execution_journal (network, account, signer, nonce)
    WHERE status = 'UNKNOWN';

CREATE INDEX idx_bulk_execution_journal_status_updated
    ON bulk_execution_journal (status, updated_at DESC);

CREATE INDEX idx_bulk_execution_journal_order_id
    ON bulk_execution_journal (network, account, computed_bulk_order_id)
    WHERE computed_bulk_order_id IS NOT NULL;

CREATE UNIQUE INDEX uq_bulk_execution_journal_idempotency
    ON bulk_execution_journal (user_id, network, idempotency_key)
    WHERE idempotency_key IS NOT NULL;

-- Per-user Bulk trading-agent wallets. The agent is used for authorized Bulk
-- trading only; native Solana settlement remains signed by the main wallet.
CREATE TABLE bulk_agent_wallets (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    turnkey_suborg_id TEXT NOT NULL,
    turnkey_wallet_id TEXT NOT NULL,
    agent_solana_pubkey TEXT NOT NULL,
    network VARCHAR(16) NOT NULL CHECK (network IN ('mainnet', 'testnet')),
    authorization_state VARCHAR(32) NOT NULL DEFAULT 'PENDING_AUTHORIZATION' CHECK (
        authorization_state IN (
            'PENDING_AUTHORIZATION',
            'AUTHORIZED',
            'REVOKED',
            'REMOVAL_PENDING',
            'REMOVED'
        )
    ),
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, network)
);

CREATE UNIQUE INDEX uq_bulk_agent_wallets_pubkey
    ON bulk_agent_wallets (network, agent_solana_pubkey);

CREATE UNIQUE INDEX uq_bulk_agent_wallets_turnkey_wallet
    ON bulk_agent_wallets (network, turnkey_suborg_id, turnkey_wallet_id);

CREATE INDEX idx_bulk_agent_wallets_state_updated
    ON bulk_agent_wallets (network, authorization_state, updated_at DESC);

-- Native Solana deposit intents. The exact unsigned message and signed
-- transaction are the replay identity because deposits have no Bulk nonce or
-- deterministic Bulk order id.
CREATE TABLE bulk_deposit_intents (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    network VARCHAR(16) NOT NULL CHECK (network IN ('mainnet', 'testnet')),
    signer TEXT NOT NULL,
    fee_payer TEXT NOT NULL,
    mint TEXT NOT NULL,
    program_id TEXT NOT NULL,
    user_token_account TEXT NOT NULL,
    vault TEXT NOT NULL,
    vault_token_account TEXT NOT NULL,
    amount_base_units TEXT NOT NULL CHECK (amount_base_units ~ '^[1-9][0-9]*$'),
    idempotency_key TEXT NOT NULL CHECK (length(btrim(idempotency_key)) BETWEEN 1 AND 255),
    idempotency_fingerprint CHAR(64) NOT NULL CHECK (idempotency_fingerprint ~ '^[0-9a-f]{64}$'),
    recent_blockhash TEXT NOT NULL,
    last_valid_block_height BIGINT,
    message_base64 TEXT NOT NULL,
    signed_transaction_base64 TEXT,
    solana_signature TEXT,
    solana_slot BIGINT,
    bulk_credit_slot BIGINT,
    status VARCHAR(32) NOT NULL CHECK (
        status IN (
            'PREPARED',
            'SUBMITTING',
            'SOLANA_FINALIZED',
            'CREDIT_PENDING',
            'CREDITED',
            'REJECTED',
            'UNKNOWN'
        )
    ),
    CONSTRAINT bulk_deposit_signature_pair_check CHECK (
        (signed_transaction_base64 IS NULL) = (solana_signature IS NULL)
    ),
    CONSTRAINT bulk_deposit_status_identity_check CHECK (
        (
            status = 'PREPARED'
            AND signed_transaction_base64 IS NULL
            AND solana_signature IS NULL
        )
        OR (
            status IN (
                'SUBMITTING',
                'SOLANA_FINALIZED',
                'CREDIT_PENDING',
                'CREDITED',
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
    CONSTRAINT uq_bulk_deposit_intents_idempotency
        UNIQUE (user_id, network, idempotency_key)
);

CREATE INDEX idx_bulk_deposit_intents_status_updated
    ON bulk_deposit_intents (network, status, updated_at DESC);

CREATE UNIQUE INDEX uq_bulk_deposit_intents_solana_signature
    ON bulk_deposit_intents (network, solana_signature)
    WHERE solana_signature IS NOT NULL;

CREATE UNIQUE INDEX uq_bulk_deposit_intents_one_active_user
    ON bulk_deposit_intents (user_id, network)
    WHERE status IN (
        'PREPARED',
        'SUBMITTING',
        'SOLANA_FINALIZED',
        'CREDIT_PENDING',
        'UNKNOWN'
    );

-- Native Solana withdrawal intents. Opcode 4 is a signal-only user
-- transaction; Bulk validators later perform the vault settlement.
CREATE TABLE bulk_withdrawal_intents (
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

CREATE INDEX idx_bulk_withdrawal_intents_status_updated
    ON bulk_withdrawal_intents (network, status, updated_at DESC);

CREATE UNIQUE INDEX uq_bulk_withdrawal_intents_solana_signature
    ON bulk_withdrawal_intents (network, solana_signature)
    WHERE solana_signature IS NOT NULL;

CREATE UNIQUE INDEX uq_bulk_withdrawal_intents_one_active_user
    ON bulk_withdrawal_intents (user_id, network)
    WHERE status IN (
        'PREPARED',
        'SUBMITTING',
        'SOLANA_FINALIZED',
        'SETTLEMENT_PENDING',
        'UNKNOWN'
    );
