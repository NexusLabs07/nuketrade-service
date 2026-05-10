-- Automation: per-user config, runs, and append-only action log.
-- Mirrors the hedge_intents/legs/tx_references reliability pattern.

CREATE TABLE IF NOT EXISTS automation_configs (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    apr_mode VARCHAR(16) NOT NULL DEFAULT 'NET',
    min_apr_to_enter DOUBLE PRECISION NOT NULL DEFAULT 0,
    exit_if_apr_below DOUBLE PRECISION NOT NULL DEFAULT 0,
    rebalance_to_better_pair BOOLEAN NOT NULL DEFAULT FALSE,
    min_rebalance_improvement_bps INT NOT NULL DEFAULT 50,
    min_time_between_actions_sec INT NOT NULL DEFAULT 300,
    cooldown_after_error_sec INT NOT NULL DEFAULT 900,
    max_position_size_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
    max_leverage DOUBLE PRECISION NOT NULL DEFAULT 1,
    max_actions_per_day INT NOT NULL DEFAULT 20,
    excluded_assets JSONB NOT NULL DEFAULT '[]'::jsonb,
    allowed_exchanges JSONB NOT NULL DEFAULT '["hyperliquid","pacifica"]'::jsonb,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS automation_runs (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'DRAFT',

    -- snapshot of config at run start (locked, not mutated by config edits)
    apr_mode_snapshot VARCHAR(16) NOT NULL,
    min_apr_to_enter_snapshot DOUBLE PRECISION NOT NULL,
    exit_if_apr_below_snapshot DOUBLE PRECISION NOT NULL,
    rebalance_to_better_pair_snapshot BOOLEAN NOT NULL,
    min_rebalance_improvement_bps_snapshot INT NOT NULL,
    min_time_between_actions_sec_snapshot INT NOT NULL,
    cooldown_after_error_sec_snapshot INT NOT NULL,
    max_position_size_usd_snapshot DOUBLE PRECISION NOT NULL,
    max_leverage_snapshot DOUBLE PRECISION NOT NULL,
    max_actions_per_day_snapshot INT NOT NULL,
    excluded_assets_snapshot JSONB NOT NULL,
    allowed_exchanges_snapshot JSONB NOT NULL,
    config_hash VARCHAR(64) NOT NULL,

    -- runtime state
    current_asset VARCHAR(40),
    current_legs JSONB,
    target_margin_usd DOUBLE PRECISION,
    leverage DOUBLE PRECISION,
    last_decision_id UUID,
    last_decision_hash VARCHAR(64),
    actions_today INT NOT NULL DEFAULT 0,
    actions_today_reset_at TIMESTAMP NOT NULL DEFAULT now(),
    last_recommendation_at TIMESTAMP,
    last_action_at TIMESTAMP,
    last_error_at TIMESTAMP,
    last_error TEXT,

    -- linkage to existing hedge intent system (Option A wiring)
    current_hedge_intent_id UUID REFERENCES hedge_intents(id) ON DELETE SET NULL,

    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

-- Enforce a single ACTIVE run per user. PAUSED/STOPPED/FAILED runs are unconstrained.
CREATE UNIQUE INDEX IF NOT EXISTS uq_automation_runs_one_active_per_user
    ON automation_runs(user_id)
    WHERE status = 'ACTIVE';

CREATE INDEX IF NOT EXISTS idx_automation_runs_user_id ON automation_runs(user_id);
CREATE INDEX IF NOT EXISTS idx_automation_runs_status ON automation_runs(status);
CREATE INDEX IF NOT EXISTS idx_automation_runs_due
    ON automation_runs(status, last_recommendation_at)
    WHERE status = 'ACTIVE';

CREATE TABLE IF NOT EXISTS automation_actions (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES automation_runs(id) ON DELETE CASCADE,
    decision_id UUID NOT NULL,
    decision_hash VARCHAR(64) NOT NULL,
    as_of_bucket BIGINT NOT NULL,
    action_type VARCHAR(20) NOT NULL,
    asset VARCHAR(40) NOT NULL,
    legs JSONB NOT NULL,
    legs_hash VARCHAR(64) NOT NULL,
    hedge_intent_id UUID REFERENCES hedge_intents(id) ON DELETE SET NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    payload JSONB,
    error TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

-- Idempotency: same decision in the same time bucket cannot create duplicate actions.
CREATE UNIQUE INDEX IF NOT EXISTS uq_automation_actions_idempotency
    ON automation_actions(run_id, action_type, asset, legs_hash, as_of_bucket);

CREATE INDEX IF NOT EXISTS idx_automation_actions_run_id ON automation_actions(run_id);
CREATE INDEX IF NOT EXISTS idx_automation_actions_status ON automation_actions(status);
