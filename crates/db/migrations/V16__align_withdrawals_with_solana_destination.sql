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

-- Terminal intents (COMPLETED / FAILED) are historical and never bridge again,
-- so they do not need a usable Solana address. Give any that could not be
-- backfilled an empty placeholder so stale rows only need to satisfy NOT NULL
-- and can never block this migration.
UPDATE withdrawal_intents
SET solana_address = ''
WHERE solana_address IS NULL
  AND status IN ('COMPLETED', 'FAILED');

-- Fail the migration rather than silently creating unusable withdrawal records,
-- but only for in-flight intents: those still need a valid origin Solana wallet
-- for later bridge execution. Full base58 validation remains the application's
-- job. Terminal intents are exempt (handled by the placeholder backfill above).
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM withdrawal_intents
        WHERE status NOT IN ('COMPLETED', 'FAILED')
          AND (solana_address IS NULL
               OR char_length(btrim(solana_address)) NOT BETWEEN 32 AND 44)
    ) THEN
        RAISE EXCEPTION
            'Cannot migrate withdrawal_intents: every in-flight intent needs a valid Solana wallet address';
    END IF;
END
$$;

-- New withdrawal intents must always retain the source Solana wallet address,
-- so later bridge execution can select the correct origin-chain account.
ALTER TABLE withdrawal_intents
    ALTER COLUMN solana_address SET NOT NULL;