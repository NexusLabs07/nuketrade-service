use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ActiveAssetCtxMsg {
    pub channel: String,
    pub data: ActiveAssetData,
}

#[derive(Debug, Deserialize)]
pub struct ActiveAssetData {
    pub coin: String,
    pub ctx: PerpCtx,
}

#[derive(Debug, Deserialize)]
pub struct PerpCtx {
    pub funding: String,
    #[serde(rename = "markPx")]
    pub mark_px: String,
}
