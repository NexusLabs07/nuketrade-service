-- Per-user Pacifica agent wallets.
--
-- Pacifica's signing model: the request body contains both `account` (the
-- master's Solana pubkey) and `agent_wallet` (the agent's Solana pubkey),
-- and the signature is produced with the agent's Ed25519 private key.
-- Pacifica technically allows one agent to be authorized by multiple
-- masters (since master is named explicitly per request), but we still
-- provision one agent per user — same shape as `hl_agent_wallets` — so
-- that a single compromised key never affects more than one user.

CREATE TABLE IF NOT EXISTS pacifica_agent_wallets (
    user_id              UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    turnkey_suborg_id    TEXT NOT NULL,
    turnkey_wallet_id    TEXT NOT NULL,
    solana_pubkey        TEXT NOT NULL,
    approved_on_pacifica BOOLEAN NOT NULL DEFAULT FALSE,
    created_at           TIMESTAMP NOT NULL DEFAULT now(),
    updated_at           TIMESTAMP NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_pacifica_agent_wallets_pubkey
    ON pacifica_agent_wallets(solana_pubkey);
