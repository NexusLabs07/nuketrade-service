CREATE TABLE IF NOT EXISTS funding_rate (
    id uuid PRIMARY KEY,
    platform VARCHAR(50) NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    rate FLOAT NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()

);