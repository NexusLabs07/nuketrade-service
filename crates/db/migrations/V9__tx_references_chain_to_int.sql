ALTER TABLE tx_references
    ALTER COLUMN chain TYPE INT USING chain::INT;
