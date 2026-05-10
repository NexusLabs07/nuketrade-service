//! HTTP controllers for the automation feature.

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use db::automation::{
    AutomationAction, AutomationConfig, AutomationRun, UpsertAutomationConfig, list_automation_actions,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::AppError,
    extractors::ValidatedJson,
    features::{
        auth::types::AuthClaims,
        automation::services::{AutomationService, parse_string_array},
    },
    services::automation::Recommendation,
    state::AppState,
};

// ============================= Request / Response =============================

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpsertAutomationConfigRequest {
    /// "NET" or "SEVEN_D" (also accepts "7D" as an alias).
    #[validate(length(min = 1, message = "aprMode must not be empty"))]
    pub apr_mode: String,
    #[validate(range(min = 0.0, message = "minAprToEnter must be >= 0"))]
    pub min_apr_to_enter: f64,
    #[validate(range(min = 0.0, message = "exitIfAprBelow must be >= 0"))]
    pub exit_if_apr_below: f64,
    pub rebalance_to_better_pair: bool,
    #[validate(range(min = 0, message = "minRebalanceImprovementBps must be >= 0"))]
    pub min_rebalance_improvement_bps: i32,
    #[validate(range(min = 0, message = "minTimeBetweenActionsSec must be >= 0"))]
    pub min_time_between_actions_sec: i32,
    #[validate(range(min = 0, message = "cooldownAfterErrorSec must be >= 0"))]
    pub cooldown_after_error_sec: i32,
    #[validate(range(min = 0.0, message = "maxPositionSizeUsd must be >= 0"))]
    pub max_position_size_usd: f64,
    #[validate(range(min = 1.0, message = "maxLeverage must be >= 1"))]
    pub max_leverage: f64,
    #[validate(range(min = 0, message = "maxActionsPerDay must be >= 0"))]
    pub max_actions_per_day: i32,
    pub excluded_assets: Vec<String>,
    pub allowed_exchanges: Option<Vec<String>>,
    /// Maximum acceptable price slippage in basis points (1 bps = 0.01%).
    /// Defaults to 50 bps when omitted.
    #[validate(range(min = 0, message = "maxSlippageBps must be >= 0"))]
    pub max_slippage_bps: Option<i32>,
    /// When closing a position, ensure orders are reduce-only so they
    /// cannot accidentally flip to a new directional position. Defaults
    /// to true.
    pub reduce_only_on_close: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationConfigResponse {
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
    pub excluded_assets: Vec<String>,
    pub allowed_exchanges: Vec<String>,
    pub max_slippage_bps: i32,
    pub reduce_only_on_close: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<AutomationConfig> for AutomationConfigResponse {
    fn from(c: AutomationConfig) -> Self {
        Self {
            user_id: c.user_id,
            apr_mode: c.apr_mode,
            min_apr_to_enter: c.min_apr_to_enter,
            exit_if_apr_below: c.exit_if_apr_below,
            rebalance_to_better_pair: c.rebalance_to_better_pair,
            min_rebalance_improvement_bps: c.min_rebalance_improvement_bps,
            min_time_between_actions_sec: c.min_time_between_actions_sec,
            cooldown_after_error_sec: c.cooldown_after_error_sec,
            max_position_size_usd: c.max_position_size_usd,
            max_leverage: c.max_leverage,
            max_actions_per_day: c.max_actions_per_day,
            excluded_assets: parse_string_array(&c.excluded_assets),
            allowed_exchanges: parse_string_array(&c.allowed_exchanges),
            max_slippage_bps: c.max_slippage_bps,
            reduce_only_on_close: c.reduce_only_on_close,
            created_at: c.created_at.and_utc().to_rfc3339(),
            updated_at: c.updated_at.and_utc().to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct BestPairQuery {
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateRunRequest {
    #[validate(range(min = 0.0, message = "targetMarginUsd must be >= 0"))]
    pub target_margin_usd: Option<f64>,
    #[validate(range(min = 1.0, message = "leverage must be >= 1"))]
    pub leverage: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRunResponse {
    pub run_id: Uuid,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRunResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub apr_mode: String,
    pub min_apr_to_enter: f64,
    pub exit_if_apr_below: f64,
    pub rebalance_to_better_pair: bool,
    pub max_leverage: f64,
    pub max_actions_per_day: i32,
    pub config_hash: String,
    pub current_asset: Option<String>,
    pub current_legs: Option<JsonValue>,
    pub target_margin_usd: Option<f64>,
    pub leverage: Option<f64>,
    pub last_decision_id: Option<Uuid>,
    pub actions_today: i32,
    pub last_recommendation_at: Option<String>,
    pub last_action_at: Option<String>,
    pub last_error_at: Option<String>,
    pub last_error: Option<String>,
    pub current_hedge_intent_id: Option<Uuid>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<AutomationRun> for AutomationRunResponse {
    fn from(r: AutomationRun) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            status: r.status,
            apr_mode: r.apr_mode_snapshot,
            min_apr_to_enter: r.min_apr_to_enter_snapshot,
            exit_if_apr_below: r.exit_if_apr_below_snapshot,
            rebalance_to_better_pair: r.rebalance_to_better_pair_snapshot,
            max_leverage: r.max_leverage_snapshot,
            max_actions_per_day: r.max_actions_per_day_snapshot,
            config_hash: r.config_hash,
            current_asset: r.current_asset,
            current_legs: r.current_legs,
            target_margin_usd: r.target_margin_usd,
            leverage: r.leverage,
            last_decision_id: r.last_decision_id,
            actions_today: r.actions_today,
            last_recommendation_at: r
                .last_recommendation_at
                .map(|t| t.and_utc().to_rfc3339()),
            last_action_at: r.last_action_at.map(|t| t.and_utc().to_rfc3339()),
            last_error_at: r.last_error_at.map(|t| t.and_utc().to_rfc3339()),
            last_error: r.last_error,
            current_hedge_intent_id: r.current_hedge_intent_id,
            created_at: r.created_at.and_utc().to_rfc3339(),
            updated_at: r.updated_at.and_utc().to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResponse {
    pub status: String,
    pub message: String,
}

// ============================= Helpers =============================

fn user_id_from_claims(claims: &AuthClaims) -> Result<Uuid, AppError> {
    Uuid::parse_str(&claims.user_id)
        .map_err(|_| AppError::unauthorised("authenticated user_id is not a valid UUID"))
}

// ============================= Handlers =============================

/// `GET /automation/config`
pub async fn get_config(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
) -> Result<Json<AutomationConfigResponse>, AppError> {
    let user_id = user_id_from_claims(&claims)?;
    let cfg = AutomationService::get_or_default_config(state.db, user_id).await?;
    Ok(Json(cfg.into()))
}

/// `PUT /automation/config`
pub async fn upsert_config(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<UpsertAutomationConfigRequest>,
) -> Result<Json<AutomationConfigResponse>, AppError> {
    let user_id = user_id_from_claims(&claims)?;
    let upsert = UpsertAutomationConfig {
        user_id,
        apr_mode: payload.apr_mode,
        min_apr_to_enter: payload.min_apr_to_enter,
        exit_if_apr_below: payload.exit_if_apr_below,
        rebalance_to_better_pair: payload.rebalance_to_better_pair,
        min_rebalance_improvement_bps: payload.min_rebalance_improvement_bps,
        min_time_between_actions_sec: payload.min_time_between_actions_sec,
        cooldown_after_error_sec: payload.cooldown_after_error_sec,
        max_position_size_usd: payload.max_position_size_usd,
        max_leverage: payload.max_leverage,
        max_actions_per_day: payload.max_actions_per_day,
        excluded_assets: serde_json::to_value(
            payload
                .excluded_assets
                .iter()
                .map(|s| s.to_uppercase())
                .collect::<Vec<_>>(),
        )
        .map_err(AppError::from)?,
        allowed_exchanges: serde_json::to_value(payload.allowed_exchanges.unwrap_or_else(|| {
            vec!["hyperliquid".to_string(), "pacifica".to_string()]
        }))
        .map_err(AppError::from)?,
        max_slippage_bps: payload.max_slippage_bps.unwrap_or(50),
        reduce_only_on_close: payload.reduce_only_on_close.unwrap_or(true),
    };
    let cfg = AutomationService::upsert_config(state.db, user_id, upsert).await?;
    Ok(Json(cfg.into()))
}

/// `GET /automation/best-pair?mode=NET|SEVEN_D`
pub async fn get_best_pair(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    Query(params): Query<BestPairQuery>,
) -> Result<Json<Recommendation>, AppError> {
    let user_id = user_id_from_claims(&claims)?;
    let rec = AutomationService::best_pair_preview(
        state.db,
        user_id,
        &state.feed,
        &state.seven_day_apr,
        params.mode,
    )
    .await?;
    Ok(Json(rec))
}

/// `POST /automation/runs`
pub async fn create_run(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateRunRequest>,
) -> Result<Json<CreateRunResponse>, AppError> {
    let user_id = user_id_from_claims(&claims)?;
    let id = AutomationService::create_run(
        state.db,
        user_id,
        payload.target_margin_usd,
        payload.leverage,
    )
    .await?;
    Ok(Json(CreateRunResponse { run_id: id }))
}

/// `POST /automation/runs/{id}/pause`
pub async fn pause_run(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<ActionResponse>, AppError> {
    let user_id = user_id_from_claims(&claims)?;
    AutomationService::get_run(state.db.clone(), user_id, run_id).await?;
    AutomationService::pause_run(state.db, run_id).await?;
    Ok(Json(ActionResponse {
        status: "ok".into(),
        message: "run paused".into(),
    }))
}

/// `POST /automation/runs/{id}/resume`
pub async fn resume_run(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<ActionResponse>, AppError> {
    let user_id = user_id_from_claims(&claims)?;
    AutomationService::get_run(state.db.clone(), user_id, run_id).await?;
    AutomationService::resume_run(state.db, run_id).await?;
    Ok(Json(ActionResponse {
        status: "ok".into(),
        message: "run resumed".into(),
    }))
}

/// `POST /automation/runs/{id}/stop`
pub async fn stop_run(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<ActionResponse>, AppError> {
    let user_id = user_id_from_claims(&claims)?;
    AutomationService::get_run(state.db.clone(), user_id, run_id).await?;
    AutomationService::stop_run(state.db, run_id).await?;
    Ok(Json(ActionResponse {
        status: "ok".into(),
        message: "run stopping".into(),
    }))
}

/// `POST /automation/runs/{id}/restart`
pub async fn restart_run(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    Path(prev_run_id): Path<Uuid>,
) -> Result<Json<CreateRunResponse>, AppError> {
    let user_id = user_id_from_claims(&claims)?;
    let new_id = AutomationService::restart_run(state.db, user_id, prev_run_id).await?;
    Ok(Json(CreateRunResponse { run_id: new_id }))
}

/// `GET /automation/runs`
pub async fn list_runs(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
) -> Result<Json<Vec<AutomationRunResponse>>, AppError> {
    let user_id = user_id_from_claims(&claims)?;
    let runs = AutomationService::list_runs(state.db, user_id).await?;
    Ok(Json(runs.into_iter().map(Into::into).collect()))
}

/// `GET /automation/runs/{id}`
pub async fn get_run(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<AutomationRunResponse>, AppError> {
    let user_id = user_id_from_claims(&claims)?;
    let run = AutomationService::get_run(state.db, user_id, run_id).await?;
    Ok(Json(run.into()))
}

/// `GET /automation/runs/{id}/actions`
pub async fn get_run_actions(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<Vec<AutomationAction>>, AppError> {
    let user_id = user_id_from_claims(&claims)?;
    AutomationService::get_run(state.db.clone(), user_id, run_id).await?;
    let actions = list_automation_actions(state.db, run_id).await?;
    Ok(Json(actions))
}
