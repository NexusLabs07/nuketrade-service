use axum::{Json, extract::Path, http::StatusCode};
use hyperliquid::{
    apis::user::{ClearinghouseState, UserInfo},
    perp_metadata::PERP_META,
    spot_metadata::SPOT_META,
};

use crate::types::OpenPositionsResponse;

//TODO: make them dynamic using cron later
pub async fn get_spot_metadata() -> &'static str {
    return SPOT_META;
}

pub async fn get_perp_metadata() -> &'static str {
    return PERP_META;
}

pub async fn get_user_open_positions(
    Path(user_evm_address): Path<String>,
) -> Result<Json<Vec<OpenPositionsResponse>>, (StatusCode, String)> {
    let user_info_client = UserInfo::new(Some(user_evm_address), None);

    let open_positions: ClearinghouseState = user_info_client
        .get_open_positions()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut open_position_response: Vec<OpenPositionsResponse> = Vec::new();

    for asset_position in open_positions.asset_positions.iter() {
        let pos = &asset_position.position;

        open_position_response.push(OpenPositionsResponse {
            symbol: pos.coin.clone(),
            size: pos.szi.clone(),
            margin: pos.margin_used.clone(),
            pnl: pos.unrealized_pnl.clone(),
            funding: pos.cum_funding.all_time.clone(),
            leverage: pos.leverage.value.clone(),
            liquidation_price: pos.liquidation_px.clone().unwrap_or_default(),
        });
    }

    Ok(Json(open_position_response))
}
