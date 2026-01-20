use hyperliquid::{perp_metadata::PERP_META, spot_metadata::SPOT_META};

//TODO: make them dynamic using cron later
pub async fn get_spot_metadata() -> &'static str {
    return SPOT_META;
}

pub async fn get_perp_metadata() -> &'static str {
    return PERP_META;
}
