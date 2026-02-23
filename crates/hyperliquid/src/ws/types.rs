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
    #[serde(rename = "dayNtlVlm")]
    pub day_ntl_vlm: String,
    #[serde(rename = "prevDayPx")]
    pub prev_day_px: String,
    #[serde(rename = "midPx")]
    pub mid_px: Option<String>,
    pub funding: String,
    #[serde(rename = "markPx")]
    pub mark_px: String,
}
