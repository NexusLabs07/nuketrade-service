use axum::{Json, extract::Path, http::StatusCode};
use pacifica::apis::user::{AccountSettingsResponse, UserInfo, UserPositionsResponse};

use crate::types::OpenPositionsResponse;

pub async fn get_user_open_positions(
    Path(user_solana_address): Path<String>,
) -> Result<Json<Vec<OpenPositionsResponse>>, (StatusCode, String)> {
    let user_info_client = UserInfo::new(user_solana_address);

    let open_positions: UserPositionsResponse = user_info_client
        .get_open_positions()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let account_setting: AccountSettingsResponse = user_info_client
        .get_account_settings()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut open_position_response: Vec<OpenPositionsResponse> = Vec::new();

    if open_positions.data.is_none()
        || open_positions.success == false
        || account_setting.success == false
        || account_setting.data.is_none()
    {
        return Ok(Json(open_position_response));
    }

    let account_setting_data = account_setting.data.unwrap();

    for asset_position in open_positions.data.unwrap().iter() {
        let account_setting = account_setting_data
            .iter()
            .find(|x| x.symbol == asset_position.symbol);

        let leverage: u32 = {
            if account_setting.is_some() {
                account_setting.unwrap().leverage.try_into().unwrap()
            } else {
                0
            }
        };

        open_position_response.push(OpenPositionsResponse {
            symbol: asset_position.symbol.clone(),
            size: asset_position.amount.clone(),
            pnl: String::from("0"), //TODO
            margin: asset_position.margin.clone().unwrap(),
            funding: asset_position.funding.clone(),
            leverage,
            liquidation_price: asset_position.liquidation_price.clone(),
        });
    }

    Ok(Json(open_position_response))
}
