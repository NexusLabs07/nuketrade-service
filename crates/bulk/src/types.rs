use serde::Deserialize;

/// Market configuration returned by Bulk's public `/exchangeInfo` endpoint.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkMarket {
    pub symbol: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub status: String,
    pub price_precision: u32,
    pub size_precision: u32,
    pub tick_size: f64,
    pub lot_size: f64,
    pub min_notional: f64,
    pub max_leverage: u32,
}

/// One externally tagged entry from Bulk's current-state account response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BulkAccountEntry {
    pub full_account: Option<BulkFullAccount>,
}

/// Read-only account snapshot returned by `type: fullAccount`.
///
/// Bulk may add unrelated account-tree, fee-tier, or order fields without
/// breaking this adapter because Serde ignores fields that are not modeled.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkFullAccount {
    pub margin: BulkMargin,

    #[serde(default)]
    pub positions: Vec<BulkPosition>,
}

/// Portfolio-level margin information returned by Bulk.
///
/// Bulk calculates margin at the portfolio level rather than treating every
/// cross-margin position as an independent collateral bucket.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkMargin {
    pub total_balance: f64,
    pub available_balance: f64,
    pub margin_used: f64,
    pub notional: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    pub fees: f64,
    pub funding: f64,
}

/// Position fields required by the shared portfolio response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkPosition {
    pub symbol: String,

    /// Signed position quantity. Positive values are long and negative values
    /// are short.
    pub size: f64,

    /// Volume-weighted position entry price.
    pub price: f64,

    /// Current fair/mark price used by Bulk's risk engine.
    pub fair_price: f64,

    pub notional: f64,
    pub unrealized_pnl: f64,
    pub leverage: f64,
    pub liquidation_price: Option<f64>,
    pub funding: f64,

    /// Position contribution to portfolio maintenance margin.
    pub maintenance_margin: f64,
}

/// Top-level WebSocket envelope.
///
/// Subscription acknowledgements and unrelated streams deserialize into this
/// shape but are ignored unless `message_type` is `ticker`.
#[derive(Debug, Deserialize)]
pub(crate) struct BulkWsMessage {
    #[serde(rename = "type")]
    pub message_type: String,

    pub data: Option<BulkTickerData>,
}

/// Payload wrapper used by Bulk ticker events.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct BulkTickerData {
    pub ticker: Option<BulkTicker>,
}

/// Market data required by the shared funding pipeline.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BulkTicker {
    pub symbol: String,
    pub mark_price: f64,
    pub funding_rate: f64,

    /// Bulk ticker events use nanoseconds even though most REST timestamps use
    /// milliseconds. Conversion happens at the exchange boundary.
    pub timestamp: i64,
}
