-- Durable HTTP idempotency for user-triggered Bulk mutations.
--
-- The journal is the shared source of truth for both the TypeScript API and
-- the Rust-owned database migrations. A nullable key preserves compatibility
-- with historical rows and with internal recovery records created before the
-- HTTP boundary required an Idempotency-Key.

ALTER TABLE bulk_execution_journal
    ADD COLUMN IF NOT EXISTS idempotency_key TEXT,
    ADD COLUMN IF NOT EXISTS idempotency_fingerprint TEXT;

ALTER TABLE bulk_execution_journal
    ADD CONSTRAINT bulk_execution_journal_idempotency_pair_check
    CHECK (
        (idempotency_key IS NULL AND idempotency_fingerprint IS NULL)
        OR
        (
            idempotency_key IS NOT NULL
            AND idempotency_fingerprint IS NOT NULL
            AND length(btrim(idempotency_key)) BETWEEN 1 AND 255
            AND length(idempotency_fingerprint) = 64
        )
    );

CREATE UNIQUE INDEX IF NOT EXISTS uq_bulk_execution_journal_idempotency
    ON bulk_execution_journal (user_id, network, idempotency_key)
    WHERE idempotency_key IS NOT NULL;
