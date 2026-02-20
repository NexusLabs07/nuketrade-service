-- Users: add name (NOT NULL), is_pacifica_access_claimed
ALTER TABLE users ADD COLUMN IF NOT EXISTS name VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_pacifica_access_claimed BOOLEAN NOT NULL DEFAULT false;

-- Wallets: make turnkey_evm_address NOT NULL, add turnkey_solana_address NOT NULL
ALTER TABLE wallets ALTER COLUMN turnkey_evm_address SET NOT NULL;
ALTER TABLE wallets ADD COLUMN IF NOT EXISTS turnkey_solana_address VARCHAR(44) UNIQUE NOT NULL DEFAULT '';
