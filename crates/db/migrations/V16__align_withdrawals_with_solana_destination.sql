-- Withdrawal intents currently store the final recipient as a Solana address.
-- Solana base58 addresses can be up to 44 characters, while the original
-- schema allowed only 42 (suitable for an EVM address).
ALTER TABLE withdrawal_intents
    ALTER COLUMN recipient TYPE VARCHAR(44);

-- Store the user's Solana address separately from the EVM address. This is
-- required when bridging USDC from Solana-based exchanges such as Pacifica.
ALTER TABLE withdrawal_intents
    ADD COLUMN IF NOT EXISTS solana_address VARCHAR(44);

-- Backfill existing records from the wallet linked to each withdrawal's user.
-- Only empty values are updated, so a previously populated address is retained.
UPDATE withdrawal_intents wi
SET solana_address = w.turnkey_solana_address
FROM users u
JOIN wallets w ON w.id = u.wallet_id
WHERE wi.user_id = u.id
  AND (wi.solana_address IS NULL OR btrim(wi.solana_address) = '');

-- Fail the migration rather than silently creating unusable withdrawal records.
-- Every existing intent must have a plausible Solana address before this column
-- becomes mandatory. Full base58 validation remains the application's job.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM withdrawal_intents
        WHERE solana_address IS NULL
           OR char_length(btrim(solana_address)) NOT BETWEEN 32 AND 44
    ) THEN
        RAISE EXCEPTION
            'Cannot migrate withdrawal_intents: every row needs a valid Solana wallet address';
    END IF;
END
$$;

-- New withdrawal intents must always retain the source Solana wallet address,
-- so later bridge execution can select the correct origin-chain account.
ALTER TABLE withdrawal_intents
    ALTER COLUMN solana_address SET NOT NULL;