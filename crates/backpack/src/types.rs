use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MarginFunction {
    #[serde(rename = "type")]
    pub function_type: String,
    pub base: String,
    pub factor: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BackpackMarket {
    pub symbol: String,
    #[serde(rename = "baseSymbol")]
    pub base_symbol: String,
    #[serde(rename = "quoteSymbol")]
    pub quote_symbol: String,
    #[serde(rename = "marketType")]
    pub market_type: String,
    #[serde(rename = "orderBookState")]
    pub order_book_state: Option<String>,
    pub visible: Option<bool>,
    #[serde(rename = "imfFunction")]
    pub imf_function: Option<MarginFunction>,
    #[serde(rename = "mmfFunction")]
    pub mmf_function: Option<MarginFunction>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamEnvelope<T> {
    pub stream: String,
    pub data: T,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MarkPriceData {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time_us: i64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub mark_price: String,
    #[serde(rename = "f")]
    pub funding_rate: Option<String>,
    #[serde(rename = "i")]
    pub index_price: Option<String>,
    #[serde(rename = "n")]
    pub next_funding_timestamp_ms: Option<i64>,
    #[serde(rename = "T")]
    pub engine_time_us: i64,
}
