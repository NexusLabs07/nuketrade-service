use axum::{
    Extension, Json,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
};
use hyperliquid::{
    metadata::{perp_metadata::PERP_META, spot_metadata::SPOT_META},
    ops::{
        deposit::{DepositPayload, deposit_to_hyperliquid},
        types::{ClearinghouseState, HyperliquidPositions},
    },
};

use crate::{
    AppState,
    error::AppError,
    extractors::{ValidatedJson, ValidatedPath},
    features::{
        auth::types::AuthClaims,
        hyperliquid::types::{HyperliquidDepositRequest, HyperliquidUserPath},
    },
    helpers::PositionService,
    types::OpenPositionsResponse,
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
    ValidatedPath(params): ValidatedPath<HyperliquidUserPath>,
) -> Result<Json<Vec<OpenPositionsResponse>>, AppError> {
    let user_info_client = HyperliquidPositions::new(Some(params.user_evm_address), None);

    let open_positions: ClearinghouseState = user_info_client.get_open_positions().await?;

    let mut open_position_response: Vec<OpenPositionsResponse> = Vec::new();

    for asset_position in &open_positions.asset_positions {
        open_position_response.push(PositionService::from_hyperliquid_position(
            &asset_position.position,
        ));
    }

    Ok(Json(open_position_response))
}

pub async fn bridge_to_hyperliquid(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<HyperliquidDepositRequest>,
) -> Result<Json<String>, AppError> {
    let arbitrum_rpc_url = &state.config.arbitrum_rpc_url;
    let fee_payer_private_key = state.config.evm_fee_payer_private_key;

    let deposit_payload = DepositPayload {
        user: claims.evm_address,
        amount: payload.amount,
        permit: payload.permit,
    };

    let tx_hash =
        deposit_to_hyperliquid(arbitrum_rpc_url, fee_payer_private_key, deposit_payload).await?;

    Ok(Json(tx_hash))
}
