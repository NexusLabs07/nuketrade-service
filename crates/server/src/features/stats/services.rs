use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use hyperliquid::apis::user::UserFill;
use lighter::apis::user::UserInfo as LighterUserInfo;
use reqwest::{Client, StatusCode, header};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

const HYPERLIQUID_MAX_FILLS_PER_PAGE: usize = 2000;
const HYPERLIQUID_MAX_SPLIT_DEPTH: u8 = 48;
const HISTORY_MAX_PAGES: usize = 10_000;

const PACIFICA_TRADES_LIMIT: u32 = 1000;
const PHOENIX_TRADES_LIMIT: u32 = 1000;
const LIGHTER_TRADES_LIMIT: u32 = 100;

#[derive(Debug, Clone, sqlx::FromRow)]
struct TurnkeyWalletRow {
    turnkey_evm_address: String,
    turnkey_solana_address: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalVolumeResponse {
    pub total_volume_usd: String,
}

pub async fn calculate_total_volume(db: Arc<PgPool>) -> Result<TotalVolumeResponse, anyhow::Error> {
    let wallets = fetch_turnkey_wallets(db).await?;
    let http = Client::new();

    let mut total = 0.0_f64;

    for wallet in wallets {
        total += calculate_wallet_volume(&http, &wallet).await?;
    }

    Ok(TotalVolumeResponse {
        total_volume_usd: format_money(total),
    })
}

async fn fetch_turnkey_wallets(db: Arc<PgPool>) -> Result<Vec<TurnkeyWalletRow>, anyhow::Error> {
    let rows = sqlx::query_as::<_, TurnkeyWalletRow>(
        r#"
        SELECT DISTINCT
            w.turnkey_evm_address,
            w.turnkey_solana_address
        FROM users u
        JOIN wallets w ON w.id = u.wallet_id
        WHERE w.turnkey_evm_address IS NOT NULL
          AND w.turnkey_solana_address IS NOT NULL
          AND btrim(w.turnkey_evm_address) <> ''
          AND btrim(w.turnkey_solana_address) <> ''
        "#,
    )
    .fetch_all(&*db)
    .await?;

    Ok(rows)
}

async fn calculate_wallet_volume(
    http: &Client,
    wallet: &TurnkeyWalletRow,
) -> Result<f64, anyhow::Error> {
    let (hyperliquid, pacifica, phoenix, lighter) = tokio::try_join!(
        hyperliquid_volume(http, &wallet.turnkey_evm_address),
        pacifica_volume(http, &wallet.turnkey_solana_address),
        phoenix_volume(http, &wallet.turnkey_solana_address),
        lighter_volume(http, &wallet.turnkey_evm_address),
    )?;

    Ok(hyperliquid + pacifica + phoenix + lighter)
}

async fn hyperliquid_volume(http: &Client, evm_address: &str) -> Result<f64, anyhow::Error> {
    let mut total = 0.0_f64;
    let mut ranges = vec![(0_i64, Utc::now().timestamp_millis(), 0_u8)];

    while let Some((start_time, end_time, depth)) = ranges.pop() {
        if start_time > end_time {
            continue;
        }

        let fills = fetch_hyperliquid_fills(http, evm_address, start_time, end_time).await?;

        if fills.len() >= HYPERLIQUID_MAX_FILLS_PER_PAGE
            && start_time < end_time
            && depth < HYPERLIQUID_MAX_SPLIT_DEPTH
        {
            let mid = start_time + ((end_time - start_time) / 2);
            ranges.push((start_time, mid, depth + 1));
            ranges.push((mid + 1, end_time, depth + 1));
            continue;
        }

        if fills.len() >= HYPERLIQUID_MAX_FILLS_PER_PAGE {
            log::warn!(
                "Hyperliquid fill range still capped for {evm_address}: {start_time}..{end_time}"
            );
        }

        total += fills.iter().map(hyperliquid_fill_notional).sum::<f64>();
    }

    Ok(total)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HyperliquidFillsByTimeRequest<'a> {
    #[serde(rename = "type")]
    request_type: &'static str,
    user: &'a str,
    start_time: i64,
    end_time: i64,
    aggregate_by_time: bool,
}

async fn fetch_hyperliquid_fills(
    http: &Client,
    evm_address: &str,
    start_time: i64,
    end_time: i64,
) -> Result<Vec<UserFill>, anyhow::Error> {
    let request = HyperliquidFillsByTimeRequest {
        request_type: "userFillsByTime",
        user: evm_address,
        start_time,
        end_time,
        aggregate_by_time: false,
    };

    let response = http
        .post(format!("{}/info", hyperliquid::HYPERLIQUID_HTTP_URL))
        .json(&request)
        .send()
        .await?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(vec![]);
    }

    if !response.status().is_success() {
        anyhow::bail!("Hyperliquid fills returned HTTP {}", response.status());
    }

    Ok(response.json::<Vec<UserFill>>().await?)
}

fn hyperliquid_fill_notional(fill: &UserFill) -> f64 {
    parse_decimal(&fill.px) * parse_decimal(&fill.sz).abs()
}

async fn pacifica_volume(http: &Client, solana_address: &str) -> Result<f64, anyhow::Error> {
    let mut total = 0.0_f64;
    let mut cursor: Option<String> = None;

    for _ in 0..HISTORY_MAX_PAGES {
        let mut request = http
            .get(format!("{}/trades/history", pacifica::PACIFICA_HTTP_URL))
            .query(&[
                ("account", solana_address.to_string()),
                ("limit", PACIFICA_TRADES_LIMIT.to_string()),
            ]);

        if let Some(ref cursor_value) = cursor {
            request = request.query(&[("cursor", cursor_value)]);
        }

        let response = request.send().await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(total);
        }

        if !response.status().is_success() {
            anyhow::bail!("Pacifica trades returned HTTP {}", response.status());
        }

        let page = response.json::<PacificaTradeHistoryResponse>().await?;

        if !page.success {
            return Ok(total);
        }

        let trades = page.data.unwrap_or_default();

        total += trades
            .iter()
            .map(|trade| parse_decimal(&trade.amount).abs() * parse_decimal(&trade.price))
            .sum::<f64>();

        cursor = page.next_cursor;

        if !page.has_more || cursor.is_none() || trades.len() < PACIFICA_TRADES_LIMIT as usize {
            return Ok(total);
        }
    }

    anyhow::bail!("Pacifica trade history exceeded page cap for {solana_address}")
}

#[derive(Debug, Deserialize)]
struct PacificaTradeHistoryResponse {
    success: bool,
    data: Option<Vec<PacificaTrade>>,
    #[serde(default)]
    next_cursor: Option<String>,
    #[serde(default)]
    has_more: bool,
}

#[derive(Debug, Deserialize)]
struct PacificaTrade {
    #[serde(default)]
    amount: String,
    #[serde(default)]
    price: String,
}

async fn phoenix_volume(http: &Client, solana_address: &str) -> Result<f64, anyhow::Error> {
    let mut total = 0.0_f64;
    let mut cursor: Option<String> = None;

    for _ in 0..HISTORY_MAX_PAGES {
        let mut request = http
            .get(format!(
                "{}/trader/{}/trades-history",
                phoenix::PHOENIX_HTTP_URL,
                solana_address
            ))
            .query(&[
                (
                    "pdaIndex",
                    phoenix::helpers::collateral::DEFAULT_TRADER_PDA_INDEX.to_string(),
                ),
                ("limit", PHOENIX_TRADES_LIMIT.to_string()),
            ]);

        if let Some(ref cursor_value) = cursor {
            request = request.query(&[("cursor", cursor_value)]);
        }

        let response = request.send().await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(total);
        }

        if !response.status().is_success() {
            anyhow::bail!("Phoenix trades returned HTTP {}", response.status());
        }

        let page = response.json::<PhoenixTradeHistoryResponse>().await?;

        total += page.data.iter().map(phoenix_trade_notional).sum::<f64>();

        cursor = page.next_cursor;

        if !page.has_more || cursor.is_none() || page.data.len() < PHOENIX_TRADES_LIMIT as usize {
            return Ok(total);
        }
    }

    anyhow::bail!("Phoenix trade history exceeded page cap for {solana_address}")
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PhoenixTradeHistoryResponse {
    #[serde(default)]
    data: Vec<PhoenixTrade>,
    #[serde(default)]
    has_more: bool,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PhoenixTrade {
    #[serde(default)]
    price: String,
    #[serde(default)]
    base_lots_delta: String,
    #[serde(default)]
    virtual_quote_lots_delta: String,
}

fn phoenix_trade_notional(trade: &PhoenixTrade) -> f64 {
    let quote = parse_decimal(&trade.virtual_quote_lots_delta).abs();
    if quote > 0.0 {
        return quote;
    }

    parse_decimal(&trade.price) * parse_decimal(&trade.base_lots_delta).abs()
}

async fn lighter_volume(http: &Client, evm_address: &str) -> Result<f64, anyhow::Error> {
    let client = LighterUserInfo::new(evm_address.to_string());
    let accounts = client.get_accounts_by_l1().await?;

    let mut total = 0.0_f64;

    for account in accounts.sub_accounts {
        total += lighter_account_volume(http, account.index).await?;
    }

    Ok(total)
}

async fn lighter_account_volume(http: &Client, account_index: u64) -> Result<f64, anyhow::Error> {
    let mut total = 0.0_f64;
    let mut cursor: Option<String> = None;

    for _ in 0..HISTORY_MAX_PAGES {
        let mut request = http
            .get(format!("{}/api/v1/trades", lighter::LIGHTER_HTTP_URL))
            .query(&[
                ("account_index", account_index.to_string()),
                ("market_type", "perp".to_string()),
                ("sort_by", "timestamp".to_string()),
                ("sort_dir", "desc".to_string()),
                ("limit", LIGHTER_TRADES_LIMIT.to_string()),
            ]);

        if let Ok(auth) = std::env::var("LIGHTER_AUTHORIZATION") {
            request = request.header(header::AUTHORIZATION, auth);
        }

        if let Some(ref cursor_value) = cursor {
            request = request.query(&[("cursor", cursor_value)]);
        }

        let response = request.send().await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(total);
        }

        if response.status() == StatusCode::UNAUTHORIZED {
            anyhow::bail!(
                "Lighter trades require Authorization for account {account_index}; set LIGHTER_AUTHORIZATION"
            );
        }

        if !response.status().is_success() {
            anyhow::bail!("Lighter trades returned HTTP {}", response.status());
        }

        let page = response.json::<LighterTradesResponse>().await?;

        total += page
            .trades
            .iter()
            .map(LighterTrade::notional_usd)
            .sum::<f64>();

        cursor = page.next_cursor.or(page.cursor);

        if cursor.is_none() || page.trades.len() < LIGHTER_TRADES_LIMIT as usize {
            return Ok(total);
        }
    }

    anyhow::bail!("Lighter trade history exceeded page cap for account {account_index}")
}

#[derive(Debug, Deserialize, Default)]
struct LighterTradesResponse {
    #[serde(
        default,
        alias = "data",
        deserialize_with = "deserialize_lighter_trades"
    )]
    trades: Vec<LighterTrade>,
    #[serde(default, alias = "nextCursor")]
    next_cursor: Option<String>,
    #[serde(default)]
    cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LighterTradesPayload {
    List(Vec<LighterTrade>),
    ByMarket(HashMap<String, Vec<LighterTrade>>),
}

fn deserialize_lighter_trades<'de, D>(deserializer: D) -> Result<Vec<LighterTrade>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let payload = LighterTradesPayload::deserialize(deserializer)?;

    Ok(match payload {
        LighterTradesPayload::List(trades) => trades,
        LighterTradesPayload::ByMarket(by_market) => by_market.into_values().flatten().collect(),
    })
}

#[derive(Debug, Clone, Deserialize, Default)]
struct LighterTrade {
    #[serde(flatten)]
    raw: Value,
}

impl LighterTrade {
    fn notional_usd(&self) -> f64 {
        let notional = first_number(
            &self.raw,
            &[
                "notional",
                "notional_usd",
                "notionalUsd",
                "quote_amount",
                "quoteAmount",
                "quote_size",
                "quoteSize",
                "usd_value",
                "usdValue",
                "value",
            ],
        )
        .abs();

        if notional > 0.0 {
            return notional;
        }

        let price = first_number(
            &self.raw,
            &["price", "px", "avg_price", "avgPrice", "execution_price"],
        );
        let size = first_number(
            &self.raw,
            &[
                "size",
                "sz",
                "amount",
                "base_amount",
                "baseAmount",
                "qty",
                "quantity",
            ],
        )
        .abs();

        price * size
    }
}

fn first_number(value: &Value, keys: &[&str]) -> f64 {
    for key in keys {
        if let Some(v) = value.get(*key) {
            let parsed = match v {
                Value::Number(n) => n.as_f64().unwrap_or(0.0),
                Value::String(s) => parse_decimal(s),
                _ => 0.0,
            };

            if parsed.abs() > f64::EPSILON {
                return parsed;
            }
        }
    }

    0.0
}

fn parse_decimal(value: &str) -> f64 {
    value.parse::<f64>().unwrap_or(0.0)
}

fn format_money(value: f64) -> String {
    format!("{value:.2}")
}
