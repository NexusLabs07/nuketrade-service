use std::sync::Arc;

use chrono::Utc;
use hyperliquid::apis::user::UserFill;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

const HYPERLIQUID_MAX_FILLS_PER_PAGE: usize = 2000;
const HYPERLIQUID_MAX_SPLIT_DEPTH: u8 = 48;
const HISTORY_MAX_PAGES: usize = 10_000;

const PACIFICA_TRADES_LIMIT: u32 = 1000;
const PHOENIX_TRADES_LIMIT: u32 = 1000;

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
    let (hyperliquid, pacifica, phoenix) = tokio::try_join!(
        hyperliquid_volume(http, &wallet.turnkey_evm_address),
        pacifica_volume(http, &wallet.turnkey_solana_address),
        phoenix_volume(http, &wallet.turnkey_solana_address),
    )?;

    Ok(hyperliquid + pacifica + phoenix)
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

fn parse_decimal(value: &str) -> f64 {
    value.parse::<f64>().unwrap_or(0.0)
}

fn format_money(value: f64) -> String {
    format!("{value:.2}")
}
