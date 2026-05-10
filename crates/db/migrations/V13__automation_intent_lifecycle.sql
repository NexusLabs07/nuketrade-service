-- Automation v2: lease + result lifecycle for the Node executor.
-- Execution moved out of Rust into an external Node service that polls
-- /internal/automation/intents/due and reports results back via
-- /internal/automation/intents/{id}/result.

-- ── automation_actions: lease + result columns ─────────────────────────────
ALTER TABLE automation_actions
    ADD COLUMN IF NOT EXISTS leased_by TEXT,
    ADD COLUMN IF NOT EXISTS leased_until TIMESTAMP,
    ADD COLUMN IF NOT EXISTS node_action_id UUID,
    ADD COLUMN IF NOT EXISTS started_at TIMESTAMP,
    ADD COLUMN IF NOT EXISTS finished_at TIMESTAMP,
    ADD COLUMN IF NOT EXISTS result_json JSONB;

-- Backfill old SUCCESS rows to the new SUCCEEDED label so app code can use
-- a single canonical value.
UPDATE automation_actions
SET status = 'SUCCEEDED'
WHERE status = 'SUCCESS';

-- Index for the /due endpoint: cheap filter on PENDING + expired-lease IN_PROGRESS.
CREATE INDEX IF NOT EXISTS idx_automation_actions_status_lease
    ON automation_actions(status, leased_until);

-- ── automation_runs: failure-tracking column ──────────────────────────────
ALTER TABLE automation_runs
    ADD COLUMN IF NOT EXISTS consecutive_failures INT NOT NULL DEFAULT 0;

-- ── automation_configs: order constraints ─────────────────────────────────
ALTER TABLE automation_configs
    ADD COLUMN IF NOT EXISTS max_slippage_bps INT NOT NULL DEFAULT 50,
    ADD COLUMN IF NOT EXISTS reduce_only_on_close BOOLEAN NOT NULL DEFAULT TRUE;
