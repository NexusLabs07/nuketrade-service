use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use hyperliquid::{
    apis::user::{ClearinghouseState, UserInfo},
    perp_metadata::PERP_META,
    services::{DepositPayload, deposit_to_hyperliquid},
    spot_metadata::SPOT_META,
};
use perp_core::parse_f64_or_zero;

use crate::{
    AppState,
    error::AppError,
    features::auth::types::AuthClaims,
    middleware::user::validate_evm_address,
    types::{OpenPositionsResponse, Side},
};

//TODO: make them dynamic using cron later
pub async fn get_spot_metadata() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        SPOT_META,
    )
}

pub async fn get_perp_metadata() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        PERP_META,
    )
}

pub async fn get_user_open_positions(
    Path(user_evm_address): Path<String>,
) -> Result<Json<Vec<OpenPositionsResponse>>, AppError> {
    validate_evm_address(&user_evm_address).map_err(|e| {
        let mut errors = validator::ValidationErrors::new();
        errors.add("user_evm_address", e);
        AppError::Validation(errors)
    })?;

    let user_info_client = UserInfo::new(Some(user_evm_address), None);

    let open_positions: ClearinghouseState = user_info_client.get_open_positions().await?;

    let mut open_position_response: Vec<OpenPositionsResponse> = Vec::new();

    for asset_position in open_positions.asset_positions.iter() {
        let pos = &asset_position.position;
        let size_value = parse_f64_or_zero(&pos.szi);
        let side = if size_value > 0.0 {
            Side::Long
        } else {
            Side::Short
        };

        open_position_response.push(OpenPositionsResponse {
            symbol: pos.coin.clone(),
            size: if side == Side::Short {
                (-size_value).to_string()
            } else {
                pos.szi.clone()
            },
            side,
            margin: pos.margin_used.clone(),
            pnl: pos.unrealized_pnl.clone(),
            funding: pos.cum_funding.all_time.clone(),
            leverage: pos.leverage.value,
            liquidation_price: pos.liquidation_px.clone().unwrap_or_default(),
        });
    }

    Ok(Json(open_position_response))
}

pub async fn bridge_to_hyperliquid(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    Json(payload): Json<DepositPayload>,
) -> Result<Json<String>, AppError> {
    validate_evm_address(&payload.user).map_err(|e| {
        let mut errors = validator::ValidationErrors::new();
        errors.add("user", e);
        AppError::Validation(errors)
    })?;

    if !payload.user.eq_ignore_ascii_case(&claims.evm_address) {
        return Err(AppError::unauthorised(
            "payload.user does not match authenticated EVM address",
        ));
    }

    let arbitrum_rpc_url = &state.config.arbitrum_rpc_url;
    let fee_payer_private_key = state.config.evm_fee_payer_private_key;

    let tx_hash = deposit_to_hyperliquid(arbitrum_rpc_url, fee_payer_private_key, payload).await?;

    Ok(Json(tx_hash))
}
