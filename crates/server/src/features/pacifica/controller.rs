use axum::{
    Extension, Json,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
};
use db::user::queries;
use pacifica::{
    apis::user::{AccountSettingsResponse, UserInfo, UserPositionsResponse},
    perp_metadata::PERP_META,
    services::deposit::{DepositPayload, deposit_to_pacifica},
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    AppState,
    error::AppError,
    extractors::{ValidatedJson, ValidatedPath},
    features::auth::types::AuthClaims,
    services::PositionService,
    types::OpenPositionsResponse,
    validation::address::validate_solana_address,
};

#[derive(Debug, Deserialize, Validate)]
pub struct PacificaUserPath {
    #[validate(custom(function = "validate_solana_address"))]
    pub user_solana_address: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct PacificaDepositRequest {
    #[validate(range(min = 1, message = "Amount must be greater than 0"))]
    pub amount: u64,
}

//TODO: make them dynamic using cron later
pub async fn get_perp_metadata() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        PERP_META,
    )
}

pub async fn get_user_open_positions(
    ValidatedPath(params): ValidatedPath<PacificaUserPath>,
    State(state): State<AppState>,
) -> Result<Json<Vec<OpenPositionsResponse>>, AppError> {
    let user_info_client = UserInfo::new(params.user_solana_address);

    let open_positions: UserPositionsResponse = user_info_client.get_open_positions().await?;
    let account_setting: AccountSettingsResponse = user_info_client.get_account_settings().await?;

    let mut open_position_response: Vec<OpenPositionsResponse> = Vec::new();

    let Some(positions_data) = open_positions.data.as_ref() else {
        return Ok(Json(open_position_response));
    };

    if !open_positions.success || !account_setting.success {
        return Ok(Json(open_position_response));
    }

    let settings_slice = account_setting.margin_settings().unwrap_or(&[]);
    let snapshot = state.feed.borrow().clone();

    for asset_position in positions_data.iter() {
        let leverage = PositionService::resolve_pacifica_leverage(
            Some(settings_slice),
            &asset_position.symbol,
        );

        let current_mark_px = snapshot
            .by_symbol
            .get(&asset_position.symbol)
            .and_then(|feed| feed.pacifica.as_ref())
            .and_then(|value| value.mark_px)
            .unwrap_or(0.0);

        let margin = if asset_position.isolated {
            asset_position.margin.clone().unwrap_or_default()
        } else {
            match asset_position.amount.parse::<f64>().ok() {
                Some(amt) if leverage > 0 => (amt * current_mark_px / leverage as f64).to_string(),
                _ => asset_position
                    .margin
                    .clone()
                    .filter(|m| !m.is_empty())
                    .unwrap_or_else(|| "0".to_string()),
            }
        };

        let pnl: f64 = if current_mark_px != 0.0 {
            let entry_price = asset_position.entry_price.parse::<f64>().unwrap_or(0.0);
            let amount = asset_position.amount.parse::<f64>().unwrap_or(0.0);
            (current_mark_px - entry_price) * amount
        } else {
            0.0
        };

        open_position_response.push(PositionService::from_pacifica_position_with_metrics(
            asset_position,
            leverage,
            margin,
            pnl,
        ));
    }

    Ok(Json(open_position_response))
}

pub async fn bridge_to_pacifica(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<PacificaDepositRequest>,
) -> Result<Json<String>, AppError> {
    let solana_rpc_url = state.config.solana_rpc_url;
    let fee_payer_private_key = state.config.solana_fee_payer_private_key;

    let deposit_payload = DepositPayload {
        user_address: claims.solana_address,
        amount: payload.amount,
    };

    let serialized_tx =
        deposit_to_pacifica(solana_rpc_url, fee_payer_private_key, deposit_payload).await?;

    Ok(Json(serialized_tx))
}
