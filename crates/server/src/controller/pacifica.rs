use axum::{Json, extract::Path, http::StatusCode};
use pacifica::apis::user::{UserInfo, UserPositionsResponse};

use crate::types::OpenPositionsResponse;

pub async fn get_user_open_positions(
    Path(user_solana_address): Path<String>,
) -> Result<Json<Vec<OpenPositionsResponse>>, (StatusCode, String)> {
    let user_info_client = UserInfo::new(user_solana_address);

    let open_positions: UserPositionsResponse = user_info_client
        .get_open_positions()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut open_position_response: Vec<OpenPositionsResponse> = Vec::new();

    if open_positions.data.is_none() || open_positions.success == false {
        return Ok(Json(open_position_response));
    }

    for asset_position in open_positions.data.unwrap().iter() {
        open_position_response.push(OpenPositionsResponse {
            symbol: asset_position.symbol.clone(),
            size: asset_position.amount.clone(),
            pnl: String::from("0"), //TODO
            funding: asset_position.funding.clone(),
            leverage: 0,                          //TODO
            liquidation_price: String::from("0"), //TODO
        });
    }

    Ok(Json(open_position_response))
}
