-- Complete the sponsored Bulk deposit identity and prevent ambiguous
-- same-user credit matching when Bulk activity history has no tx signature.
--
-- V20/V21 may already have been applied before the sponsor-wallet change.
-- Existing rows were created by the previous implementation with the user as
-- both fee payer and transfer authority, so backfill that historical identity
-- explicitly. New rows are written with the NukeTrade sponsor wallet.

ALTER TABLE bulk_deposit_intents
    ADD COLUMN IF NOT EXISTS fee_payer TEXT;

UPDATE bulk_deposit_intents
SET fee_payer = signer
WHERE fee_payer IS NULL;

ALTER TABLE bulk_deposit_intents
    ALTER COLUMN fee_payer SET NOT NULL;

-- Bulk activity history does not expose the originating Solana transaction
-- signature. Serialize one non-terminal deposit per user and network so two
-- equal-amount deposits cannot both claim the same activity row as credit.
CREATE UNIQUE INDEX IF NOT EXISTS uq_bulk_deposit_intents_one_active_user
    ON bulk_deposit_intents (user_id, network)
    WHERE status IN (
        'PREPARED',
        'SUBMITTING',
        'SOLANA_FINALIZED',
        'CREDIT_PENDING',
        'UNKNOWN'
    );
