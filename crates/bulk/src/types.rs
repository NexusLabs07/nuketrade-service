use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkMarket {
    pub symbol: String,
    pub status: String,
    pub max_leverage: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BulkWsMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub topic: Option<String>,
    pub symbol: Option<String>,
    pub data: Option<BulkTickerData>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkTickerData {
    pub symbol: Option<String>,
    pub ticker: Option<BulkTicker>,
    pub mark_price: Option<f64>,
    pub funding_rate: Option<f64>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkTicker {
    pub symbol: Option<String>,
    pub mark_price: Option<f64>,
    pub funding_rate: Option<f64>,
    pub timestamp: Option<i64>,
}
