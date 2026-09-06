-- Durable Bulk nonce allocation and signed-request journal.
--
-- These tables intentionally do not share ownership with the automation
-- subsystem. They record the identity-scoped nonce and the exact request
-- lifecycle needed to recover an ambiguous signed submission safely.

CREATE TABLE IF NOT EXISTS bulk_nonce_state (
    account TEXT NOT NULL,
    signer TEXT NOT NULL,
    network VARCHAR(16) NOT NULL CHECK (network IN ('mainnet', 'testnet')),
    last_nonce BIGINT NOT NULL CHECK (last_nonce >= 0),
    updated_at TIMESTAMP NOT NULL DEFAULT now(),
    PRIMARY KEY (account, signer, network)
);

CREATE TABLE IF NOT EXISTS bulk_execution_journal (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    account TEXT NOT NULL,
    signer TEXT NOT NULL,
    network VARCHAR(16) NOT NULL CHECK (network IN ('mainnet', 'testnet')),
    nonce BIGINT NOT NULL CHECK (nonce >= 0),
    canonical_request_hash CHAR(64) NOT NULL,
    computed_bulk_order_id TEXT,
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
    UNIQUE (account, signer, network, nonce)
);

CREATE INDEX IF NOT EXISTS idx_bulk_execution_journal_user_created
    ON bulk_execution_journal (user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_bulk_execution_journal_unknown
    ON bulk_execution_journal (network, account, signer, nonce)
    WHERE status = 'UNKNOWN';

CREATE INDEX IF NOT EXISTS idx_bulk_execution_journal_status_updated
    ON bulk_execution_journal (status, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_bulk_execution_journal_order_id
    ON bulk_execution_journal (network, account, computed_bulk_order_id)
    WHERE computed_bulk_order_id IS NOT NULL;
