-- Enforce the persisted transaction identity required by the Bulk deposit
-- recovery state machine. V20 creates these checks on a new database; this
-- migration is idempotent for databases where V20 was already applied before
-- the checks were added.

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'bulk_deposit_intents'::regclass
          AND conname = 'bulk_deposit_signature_pair_check'
    ) THEN
        ALTER TABLE bulk_deposit_intents
            ADD CONSTRAINT bulk_deposit_signature_pair_check CHECK (
                (signed_transaction_base64 IS NULL) = (solana_signature IS NULL)
            );
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'bulk_deposit_intents'::regclass
          AND conname = 'bulk_deposit_status_identity_check'
    ) THEN
        ALTER TABLE bulk_deposit_intents
            ADD CONSTRAINT bulk_deposit_status_identity_check CHECK (
                (
                    status = 'PREPARED'
                    AND signed_transaction_base64 IS NULL
                    AND solana_signature IS NULL
                )
                OR (
                    status IN (
                        'SUBMITTING',
                        'SOLANA_FINALIZED',
                        'CREDIT_PENDING',
                        'CREDITED',
                        'UNKNOWN'
                    )
                    AND signed_transaction_base64 IS NOT NULL
                    AND solana_signature IS NOT NULL
                )
                OR status = 'REJECTED'
            );
    END IF;
END $$;
