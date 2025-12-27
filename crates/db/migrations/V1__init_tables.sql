CREATE TABLE funding_rate (
    id uuid PRIMARY KEY,
    platform VARCHAR(50) NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    rate FLOAT NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL

);