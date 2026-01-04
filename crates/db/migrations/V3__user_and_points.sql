CREATE TABLE IF NOT EXISTS wallets (
    id uuid PRIMARY KEY,
    turnkey_evm_address VARCHAR(42) UNIQUE NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS users (
    id uuid PRIMARY KEY,
    email VARCHAR(50) UNIQUE,
    connected_evm_address VARCHAR(42) UNIQUE,
    connected_solana_address VARCHAR(44) UNIQUE,
    referral_code VARCHAR(50) UNIQUE NOT NULL,
    referred_by uuid,
    wallet_id uuid,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now(),
    CONSTRAINT fk_referred_by
        FOREIGN KEY (referred_by)
        REFERENCES users(id)
        ON DELETE SET NULL,
    CONSTRAINT fk_wallet
        FOREIGN KEY (wallet_id)
        REFERENCES wallets(id)
        ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS points (
    id UUID PRIMARY KEY,
    user_id UUID UNIQUE NOT NULL,
    total_points INT NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now(),

    CONSTRAINT fk_user_points
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE
);

CREATE INDEX idx_users_referred_by ON users(referred_by);
