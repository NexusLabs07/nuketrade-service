use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    PHOENIX_HTTP_URL,
    helpers::collateral::{DEFAULT_TRADER_PDA_INDEX, PhoenixAmount},
};

#[derive(Debug, Clone)]
pub struct UserInfo {
    client: Client,
    http_url: String,
    authority: String,
    pda_index: Option<u32>,
}

impl UserInfo {
    pub fn new(authority: String) -> Self {
        Self {
            client: Client::new(),
            http_url: PHOENIX_HTTP_URL.to_string(),
            authority,
            pda_index: None,
        }
    }

    pub fn with_pda_index(authority: String, pda_index: u32) -> Self {
        Self {
            client: Client::new(),
            http_url: PHOENIX_HTTP_URL.to_string(),
            authority,
            pda_index: Some(pda_index),
        }
    }

    pub fn with_url(authority: String, http_url: String) -> Self {
        Self {
            client: Client::new(),
            http_url,
            authority,
            pda_index: None,
        }
    }

    fn with_pda_query(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self.pda_index {
            Some(pda_index) => request.query(&[("pdaIndex", pda_index)]),
            None => request,
        }
    }

    pub async fn get_trader_state(&self) -> Result<TraderStateResponse> {
        self.get_trader_state_for_pda(self.pda_index.unwrap_or(DEFAULT_TRADER_PDA_INDEX))
            .await
    }

    /// Trader state for a specific PDA index (FE default: `0`).
    pub async fn get_trader_state_for_pda(&self, pda_index: u32) -> Result<TraderStateResponse> {
        let request = self
            .client
            .get(format!("{}/trader/{}/state", self.http_url, self.authority))
            .query(&[("pdaIndex", pda_index)]);

        let response = request.send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Phoenix trader state returned HTTP {}", response.status());
        }

        Ok(response.json::<TraderStateResponse>().await?)
    }

    pub async fn get_trade_history(&self) -> Result<TradeHistoryResponse> {
        let request = self.client.get(format!(
            "{}/trader/{}/trades-history",
            self.http_url, self.authority
        ));

        let response = self.with_pda_query(request).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Phoenix trade history returned HTTP {}", response.status());
        }

        Ok(response.json::<TradeHistoryResponse>().await?)
    }

    pub async fn get_funding_history(&self) -> Result<FundingHistoryResponse> {
        let request = self.client.get(format!(
            "{}/trader/{}/funding-history",
            self.http_url, self.authority
        ));

        let response = self.with_pda_query(request).send().await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Phoenix funding history returned HTTP {}",
                response.status()
            );
        }

        Ok(response.json::<FundingHistoryResponse>().await?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateResponse {
    pub authority: String,
    pub pda_index: u32,
    pub slot: i64,
    pub slot_index: i64,
    #[serde(default)]
    pub traders: Vec<PhoenixTrader>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoenixTrader {
    pub authority: String,
    pub trader_key: String,
    pub trader_pda_index: u32,
    pub trader_subaccount_index: u32,

    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub risk_state: String,
    #[serde(default)]
    pub risk_tier: String,

    #[serde(default)]
    pub collateral_balance: PhoenixAmount,
    #[serde(default)]
    pub effective_collateral: PhoenixAmount,
    #[serde(default)]
    pub effective_collateral_for_withdrawals: PhoenixAmount,
    #[serde(default)]
    pub portfolio_value: PhoenixAmount,
    #[serde(default)]
    pub initial_margin: PhoenixAmount,
    #[serde(default)]
    pub maintenance_margin: PhoenixAmount,
    #[serde(default)]
    pub unrealized_pnl: PhoenixAmount,
    #[serde(default)]
    pub unsettled_funding_owed: PhoenixAmount,
    #[serde(default)]
    pub accumulated_funding: PhoenixAmount,

    #[serde(default)]
    pub positions: Vec<PhoenixPosition>,

    #[serde(default)]
    pub limit_orders: Value,
}

impl PhoenixTrader {
    /// Deposited collateral on this subaccount (matches Phoenix UI isolated margin).
    pub fn collateral_usd(&self) -> f64 {
        let balance = self.collateral_balance.to_usd();
        if balance > 0.0 {
            balance
        } else {
            self.effective_collateral.to_usd()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoenixPosition {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub position_size: PhoenixAmount,
    #[serde(default)]
    pub virtual_quote_position: PhoenixAmount,
    #[serde(default)]
    pub entry_price: PhoenixAmount,
    #[serde(default)]
    pub liquidation_price: PhoenixAmount,
    #[serde(default)]
    pub position_initial_margin: PhoenixAmount,
    #[serde(default)]
    pub initial_margin: PhoenixAmount,
    #[serde(default)]
    pub maintenance_margin: PhoenixAmount,
    #[serde(default)]
    pub position_value: PhoenixAmount,
    #[serde(default)]
    pub unrealized_pnl: PhoenixAmount,
    #[serde(default)]
    pub unsettled_funding: PhoenixAmount,
    #[serde(default)]
    pub accumulated_funding: PhoenixAmount,
}

impl PhoenixPosition {
    /// Signed base position size in UI units (e.g. 0.02 ZEC). Positive = long, negative = short.
    ///
    /// Phoenix docs: `unrealized_pnl = position_size * (mark_price - entry_price)`.
    /// REST often exposes `positionSize` as a positive magnitude with
    /// `virtualQuotePosition` negative even for longs, so when signs disagree we
    /// infer side from entry, mark (via `positionValue`), and unrealized PnL.
    pub fn signed_position_size(&self) -> f64 {
        let base = self.position_size.to_f64();
        if base.abs() < f64::EPSILON {
            return 0.0;
        }

        if self.position_size.value < 0 {
            return base;
        }

        let magnitude = base.abs();
        let quote = self.virtual_quote_position.to_f64();
        let raw_base = self.position_size.value;

        if raw_base > 0 && quote > 0.0 {
            return magnitude;
        }
        if raw_base < 0 && quote < 0.0 {
            return -magnitude;
        }

        self.infer_signed_size_from_pnl(magnitude)
    }

    /// Infer `sign(position_size)` from Phoenix PnL identity when REST omits a sign.
    fn infer_signed_size_from_pnl(&self, magnitude: f64) -> f64 {
        let entry = self.entry_price.to_f64();
        let pnl = self.unrealized_pnl.to_f64();
        let notional = self.position_value.to_f64();

        if entry <= 0.0 || notional <= 0.0 {
            return magnitude;
        }

        let mark = notional / magnitude;
        let delta = mark - entry;
        if delta.abs() < 1e-12 {
            return magnitude;
        }
        if pnl.abs() < 1e-12 {
            return magnitude;
        }

        // unrealized_pnl = q * (mark - entry); solve sign(q)
        if (pnl > 0.0) == (delta > 0.0) {
            magnitude
        } else {
            -magnitude
        }
    }

    pub fn margin_usd(&self) -> f64 {
        let m = self.position_initial_margin.to_usd();
        if m > 0.0 {
            return m;
        }
        self.initial_margin.to_usd()
    }

    pub fn funding_usd(&self) -> f64 {
        let acc = self.accumulated_funding.to_f64();
        if acc.abs() >= f64::EPSILON {
            return acc;
        }
        self.unsettled_funding.to_f64()
    }

    pub fn leverage_from_margin(&self) -> u32 {
        let margin = self.margin_usd();
        let notional = self.position_value.to_usd();
        if margin > 0.0 && notional > 0.0 {
            return (notional / margin).round().max(1.0) as u32;
        }
        0
    }

    /// Margin shown in Phoenix UI for isolated positions: subaccount collateral, not
    /// `positionInitialMargin` (exchange margin requirement at max tier).
    pub fn display_margin_usd(&self, subaccount_collateral_usd: f64, single_position: bool) -> f64 {
        if single_position && subaccount_collateral_usd > 0.0 {
            return subaccount_collateral_usd;
        }
        self.margin_usd()
    }

    pub fn display_leverage(&self, subaccount_collateral_usd: f64, single_position: bool) -> u32 {
        let margin = self.display_margin_usd(subaccount_collateral_usd, single_position);
        let notional = self.position_value.to_usd();
        if margin > 0.0 && notional > 0.0 {
            return (notional / margin).round().max(1.0) as u32;
        }
        0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeHistoryResponse {
    #[serde(default)]
    pub data: Vec<PhoenixTrade>,
    #[serde(default)]
    pub has_more: bool,
    pub next_cursor: Option<String>,
    pub prev_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoenixTrade {
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub market_symbol: String,
    #[serde(default)]
    pub price: String,
    #[serde(default)]
    pub realized_pnl: String,
    #[serde(default)]
    pub base_lots_delta: String,
    #[serde(default)]
    pub virtual_quote_lots_delta: String,
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingHistoryResponse {
    #[serde(default)]
    pub events: Vec<PhoenixFundingEvent>,
    #[serde(default)]
    pub has_more: bool,
    pub next_cursor: Option<String>,
    pub prev_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoenixFundingEvent {
    #[serde(default)]
    pub funding_payment: String,
    #[serde(default)]
    pub funding_rate_percentage: String,
    #[serde(default)]
    pub position_side: String,
    #[serde(default)]
    pub position_size: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub timestamp: String,
}
