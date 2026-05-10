use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

pub mod apr_mode {
    pub const NET: &str = "NET";
    pub const SEVEN_D: &str = "SEVEN_D";
}

pub mod run_status {
    pub const DRAFT: &str = "DRAFT";
    pub const ACTIVE: &str = "ACTIVE";
    pub const PAUSED: &str = "PAUSED";
    pub const STOPPING: &str = "STOPPING";
    pub const STOPPED: &str = "STOPPED";
    pub const FAILED: &str = "FAILED";
}

pub mod action_type {
    pub const OPEN: &str = "OPEN";
    pub const CLOSE: &str = "CLOSE";
    pub const REBALANCE: &str = "REBALANCE";
    pub const EMERGENCY_CLOSE: &str = "EMERGENCY_CLOSE";
}

pub mod action_status {
    pub const PENDING: &str = "PENDING";
    pub const IN_PROGRESS: &str = "IN_PROGRESS";
    pub const SUCCEEDED: &str = "SUCCEEDED";
    pub const FAILED: &str = "FAILED";
    pub const PARTIAL_FAILURE: &str = "PARTIAL_FAILURE";
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct AutomationConfig {
    pub user_id: Uuid,
    pub apr_mode: String,
    pub min_apr_to_enter: f64,
    pub exit_if_apr_below: f64,
    pub rebalance_to_better_pair: bool,
    pub min_rebalance_improvement_bps: i32,
    pub min_time_between_actions_sec: i32,
    pub cooldown_after_error_sec: i32,
    pub max_position_size_usd: f64,
    pub max_leverage: f64,
    pub max_actions_per_day: i32,
    pub excluded_assets: JsonValue,
    pub allowed_exchanges: JsonValue,
    pub max_slippage_bps: i32,
    pub reduce_only_on_close: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl AutomationConfig {
    /// Default config row for a new user. The DB also has these defaults but
    /// having them in code lets callers prefill UI without an INSERT first.
    pub fn defaults_for(user_id: Uuid) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            user_id,
            apr_mode: apr_mode::NET.to_string(),
            min_apr_to_enter: 0.0,
            exit_if_apr_below: 0.0,
            rebalance_to_better_pair: false,
            min_rebalance_improvement_bps: 50,
            min_time_between_actions_sec: 300,
            cooldown_after_error_sec: 900,
            max_position_size_usd: 0.0,
            max_leverage: 1.0,
            max_actions_per_day: 20,
            excluded_assets: serde_json::json!([]),
            allowed_exchanges: serde_json::json!(["hyperliquid", "pacifica"]),
            max_slippage_bps: 50,
            reduce_only_on_close: true,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpsertAutomationConfig {
    pub user_id: Uuid,
    pub apr_mode: String,
    pub min_apr_to_enter: f64,
    pub exit_if_apr_below: f64,
    pub rebalance_to_better_pair: bool,
    pub min_rebalance_improvement_bps: i32,
    pub min_time_between_actions_sec: i32,
    pub cooldown_after_error_sec: i32,
    pub max_position_size_usd: f64,
    pub max_leverage: f64,
    pub max_actions_per_day: i32,
    pub excluded_assets: JsonValue,
    pub allowed_exchanges: JsonValue,
    pub max_slippage_bps: i32,
    pub reduce_only_on_close: bool,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct AutomationRun {
    pub id: Uuid,
    pub user_id: Uuid,
    pub status: String,

    pub apr_mode_snapshot: String,
    pub min_apr_to_enter_snapshot: f64,
    pub exit_if_apr_below_snapshot: f64,
    pub rebalance_to_better_pair_snapshot: bool,
    pub min_rebalance_improvement_bps_snapshot: i32,
    pub min_time_between_actions_sec_snapshot: i32,
    pub cooldown_after_error_sec_snapshot: i32,
    pub max_position_size_usd_snapshot: f64,
    pub max_leverage_snapshot: f64,
    pub max_actions_per_day_snapshot: i32,
    pub excluded_assets_snapshot: JsonValue,
    pub allowed_exchanges_snapshot: JsonValue,
    pub config_hash: String,

    pub current_asset: Option<String>,
    pub current_legs: Option<JsonValue>,
    pub target_margin_usd: Option<f64>,
    pub leverage: Option<f64>,
    pub last_decision_id: Option<Uuid>,
    pub last_decision_hash: Option<String>,
    pub actions_today: i32,
    pub actions_today_reset_at: NaiveDateTime,
    pub last_recommendation_at: Option<NaiveDateTime>,
    pub last_action_at: Option<NaiveDateTime>,
    pub last_error_at: Option<NaiveDateTime>,
    pub last_error: Option<String>,
    pub consecutive_failures: i32,

    /// DEPRECATED: kept for backward compat with manual hedge intents. The
    /// automation flow no longer creates hedge_intents — execution is
    /// delegated to an external Node executor via `/internal/automation/*`.
    pub current_hedge_intent_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct NewAutomationRun {
    pub id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub config: AutomationConfig,
    pub config_hash: String,
    pub target_margin_usd: Option<f64>,
    pub leverage: Option<f64>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct AutomationAction {
    pub id: Uuid,
    pub run_id: Uuid,
    pub decision_id: Uuid,
    pub decision_hash: String,
    pub as_of_bucket: i64,
    pub action_type: String,
    pub asset: String,
    pub legs: JsonValue,
    pub legs_hash: String,
    /// DEPRECATED: see `AutomationRun::current_hedge_intent_id`.
    pub hedge_intent_id: Option<Uuid>,
    pub status: String,
    pub payload: Option<JsonValue>,
    pub error: Option<String>,
    /// Worker id of the lease holder (Node side). NULL when not leased.
    pub leased_by: Option<String>,
    /// Wall-clock UTC at which the lease expires. NULL when not leased.
    pub leased_until: Option<NaiveDateTime>,
    pub node_action_id: Option<Uuid>,
    pub started_at: Option<NaiveDateTime>,
    pub finished_at: Option<NaiveDateTime>,
    pub result_json: Option<JsonValue>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct NewAutomationAction {
    pub id: Uuid,
    pub run_id: Uuid,
    pub decision_id: Uuid,
    pub decision_hash: String,
    pub as_of_bucket: i64,
    pub action_type: String,
    pub asset: String,
    pub legs: JsonValue,
    pub legs_hash: String,
    pub hedge_intent_id: Option<Uuid>,
    pub payload: Option<JsonValue>,
}

/// Result of inspecting/leasing an action row in the `/due` endpoint.
#[derive(Debug, Clone)]
pub enum LeaseOutcome {
    /// We successfully leased the action; caller should return it.
    Leased(AutomationAction),
    /// Already in a terminal state — caller should skip.
    Terminal,
    /// Currently leased by someone else (lease not yet expired).
    BusyElsewhere,
}
