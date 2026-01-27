use axum::{Json, extract::Path};
use hyperliquid::{
    apis::user::{ClearinghouseState, UserInfo},
    perp_metadata::PERP_META,
    spot_metadata::SPOT_META,
};

use crate::{
    error::AppError,
    types::{OpenPositionsResponse, Side},
};

//TODO: make them dynamic using cron later
pub async fn get_spot_metadata() -> &'static str {
    SPOT_META
}

pub async fn get_perp_metadata() -> &'static str {
    PERP_META
}

pub async fn get_user_open_positions(
    Path(user_evm_address): Path<String>,
) -> Result<Json<Vec<OpenPositionsResponse>>, AppError> {
    let user_info_client = UserInfo::new(Some(user_evm_address), None);

    let open_positions: ClearinghouseState = user_info_client.get_open_positions().await?;

    let mut open_position_response: Vec<OpenPositionsResponse> = Vec::new();

    for asset_position in open_positions.asset_positions.iter() {
        let pos = &asset_position.position;
        let side = if pos.szi.parse::<f64>().unwrap() > 0.0 {
            Side::Long
        } else {
            Side::Short
        };

        open_position_response.push(OpenPositionsResponse {
            symbol: pos.coin.clone(),
            size: if side == Side::Short {
                (-pos.szi.parse::<f64>().unwrap()).to_string().clone()
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
