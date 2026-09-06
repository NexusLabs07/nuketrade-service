-- Per-user Bulk trading-agent wallets.
--
-- Bulk authorizes a Solana agent public key against a master account. The
-- Turnkey wallet is separate from the user's main wallet, while the current
-- process network remains part of the identity so testnet and mainnet state
-- cannot be mixed accidentally.

CREATE TABLE IF NOT EXISTS bulk_agent_wallets (
    user_id              UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    turnkey_suborg_id    TEXT NOT NULL,
    turnkey_wallet_id    TEXT NOT NULL,
    agent_solana_pubkey  TEXT NOT NULL,
    network              VARCHAR(16) NOT NULL CHECK (network IN ('mainnet', 'testnet')),
    authorization_state  VARCHAR(32) NOT NULL DEFAULT 'PENDING_AUTHORIZATION' CHECK (
        authorization_state IN (
            'PENDING_AUTHORIZATION',
            'AUTHORIZED',
            'REVOKED',
            'REMOVAL_PENDING',
            'REMOVED'
        )
    ),
    created_at           TIMESTAMP NOT NULL DEFAULT now(),
    updated_at           TIMESTAMP NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, network)
);

-- One agent key may act for only one NukeTrade user within a Bulk network.
CREATE UNIQUE INDEX IF NOT EXISTS uq_bulk_agent_wallets_pubkey
    ON bulk_agent_wallets (network, agent_solana_pubkey);

-- A Turnkey wallet must not be accidentally attached to multiple users in one
-- network, even if a caller supplies a stale or conflicting local record.
CREATE UNIQUE INDEX IF NOT EXISTS uq_bulk_agent_wallets_turnkey_wallet
    ON bulk_agent_wallets (network, turnkey_suborg_id, turnkey_wallet_id);

-- Verification and operational recovery scans use the lifecycle state and
-- update timestamp together.
CREATE INDEX IF NOT EXISTS idx_bulk_agent_wallets_state_updated
    ON bulk_agent_wallets (network, authorization_state, updated_at DESC);
