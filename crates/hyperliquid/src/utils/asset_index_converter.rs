use anyhow::{Result, anyhow};

use crate::utils::market_price::HyperliquidMarketPrice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketType {
    Spot,
    Perp,
}

/// Convert perpetual asset index to ticker name ("ETH", "BTC", etc.)
pub async fn perp_index_to_ticker(asset_index: usize) -> Result<String> {
    let market_price = HyperliquidMarketPrice::new();
    let perp_meta = market_price.get_perp_meta_public().await?;

    let ticker = perp_meta
        .get(asset_index)
        .map(|meta| meta.name.clone())
        .ok_or_else(|| anyhow!("Asset index {} not found in perp meta", asset_index))?;

    Ok(ticker)
}

/// Convert perpetual ticker name to asset index
pub async fn perp_ticker_to_index(ticker: &str) -> Result<usize> {
    let market_price = HyperliquidMarketPrice::new();
    let perp_meta = market_price.get_perp_meta_public().await?;

    let ticker_upper = ticker.to_uppercase();
    let asset_index = perp_meta
        .iter()
        .position(|meta| meta.name == ticker_upper)
        .ok_or_else(|| anyhow!("Ticker {} not found in perp meta", ticker))?;

    Ok(asset_index)
}

/// Convert spot ticker to asset index
/// Spot asset index = 10000 + market index
pub async fn spot_ticker_to_index(ticker: &str, quote_token: Option<&str>) -> Result<u32> {
    // Special case for PURR/USDC
    if ticker == "PURR/USDC" {
        return Ok(10000);
    }

    let quote_token = quote_token.unwrap_or("USDC");
    let ticker_upper = ticker.to_uppercase();

    // Map ticker names
    let formatted_ticker = match ticker_upper.as_str() {
        "ETH" => "UETH".to_string(),
        "BTC" => "UBTC".to_string(),
        "SOL" => "USOL".to_string(),
        _ => ticker_upper,
    };

    let market_price = HyperliquidMarketPrice::new();
    let spot_meta = market_price.get_spot_meta_public().await?;

    // Find token indices
    let base_index = spot_meta
        .tokens
        .iter()
        .position(|t| t.name == formatted_ticker)
        .ok_or_else(|| anyhow!("Base token {} not found in spot meta", formatted_ticker))?;

    let quote_index = spot_meta
        .tokens
        .iter()
        .position(|t| t.name == quote_token.to_uppercase())
        .ok_or_else(|| anyhow!("Quote token {} not found in spot meta", quote_token))?;

    // Find matching universe entry
    let market = spot_meta
        .universe
        .iter()
        .find(|m| m.tokens[0] == base_index && m.tokens[1] == quote_index)
        .ok_or_else(|| {
            anyhow!(
                "No market found for {}/{} in spot universe",
                formatted_ticker,
                quote_token
            )
        })?;

    Ok(10000 + market.index)
}

/// Convert spot asset index to ticker (e.g., "ETH/USDC")
pub async fn spot_index_to_ticker(asset_index: u32) -> Result<String> {
    let index = asset_index - 10000;

    let market_price = HyperliquidMarketPrice::new();
    let spot_meta = market_price.get_spot_meta_public().await?;

    let market = spot_meta
        .universe
        .iter()
        .find(|m| m.index == index)
        .ok_or_else(|| anyhow!("Market index {} not found in spot universe", index))?;

    let base_token = spot_meta
        .tokens
        .get(market.tokens[0])
        .ok_or_else(|| anyhow!("Base token index {} not found", market.tokens[0]))?;

    let quote_token = spot_meta
        .tokens
        .get(market.tokens[1])
        .ok_or_else(|| anyhow!("Quote token index {} not found", market.tokens[1]))?;

    Ok(format!("{}/{}", base_token.name, quote_token.name))
}

/// Convert spot ticker to @ symbol format (e.g., "@87")
pub async fn spot_ticker_to_at_symbol(ticker: &str, quote_token: Option<&str>) -> Result<String> {
    let quote_token = quote_token.unwrap_or("USDC");

    let market_price = HyperliquidMarketPrice::new();
    let spot_meta = market_price.get_spot_meta_public().await?;

    let base_index = spot_meta
        .tokens
        .iter()
        .position(|t| t.name == ticker.to_uppercase())
        .ok_or_else(|| anyhow!("Base token {} not found in spot meta", ticker))?;

    let quote_index = spot_meta
        .tokens
        .iter()
        .position(|t| t.name == quote_token.to_uppercase())
        .ok_or_else(|| anyhow!("Quote token {} not found in spot meta", quote_token))?;

    let market = spot_meta
        .universe
        .iter()
        .find(|m| m.tokens[0] == base_index && m.tokens[1] == quote_index)
        .ok_or_else(|| {
            anyhow!(
                "No market found for {}/{} in spot universe",
                ticker,
                quote_token
            )
        })?;

    Ok(format!("@{}", market.index))
}

/// Returns the key to use when accessing allMids[ticker]
/// For perps: returns "ETH", "BTC", etc.
/// For spot: returns "@87", "@123", etc.
pub async fn ticker_to_all_mids_key(ticker: &str, quote_token: Option<&str>) -> Result<String> {
    let quote_token = quote_token.unwrap_or("USDC");

    let market_price = HyperliquidMarketPrice::new();

    // Check if it's a perp
    let perp_meta = market_price.get_perp_meta_public().await?;
    let ticker_upper = ticker.to_uppercase();

    if perp_meta.iter().any(|p| p.name == ticker_upper) {
        return Ok(ticker_upper);
    }

    // Otherwise, treat as spot
    let spot_meta = market_price.get_spot_meta_public().await?;

    let base_index = spot_meta
        .tokens
        .iter()
        .position(|t| t.name == ticker_upper)
        .ok_or_else(|| anyhow!("Ticker {} not found in spot or perp meta", ticker))?;

    let quote_index = spot_meta
        .tokens
        .iter()
        .position(|t| t.name == quote_token.to_uppercase())
        .ok_or_else(|| anyhow!("Quote token {} not found in spot meta", quote_token))?;

    let spot_match = spot_meta
        .universe
        .iter()
        .find(|m| m.tokens[0] == base_index && m.tokens[1] == quote_index)
        .ok_or_else(|| anyhow!("No spot market found for {}/{}", ticker, quote_token))?;

    Ok(format!("@{}", spot_match.index))
}

/// Extract number from @ symbol (e.g., "@87" -> 87)
fn extract_number(input: &str) -> Result<u32> {
    let num_str = input.trim_start_matches('@');
    num_str
        .parse::<u32>()
        .map_err(|_| anyhow!("Invalid @ symbol format: {}", input))
}

/// Convert @ symbol to ticker name (e.g., "@87" -> "ETH")
pub async fn at_symbol_to_ticker(at_symbol: &str) -> Result<String> {
    let market_index = extract_number(at_symbol)?;

    let market_price = HyperliquidMarketPrice::new();
    let spot_meta = market_price.get_spot_meta_public().await?;

    // Find the universe entry with the market index
    let market_info = spot_meta
        .universe
        .iter()
        .find(|market| market.index == market_index)
        .ok_or_else(|| anyhow!("Market index {} not found in spot universe", market_index))?;

    // Get the base token index (first element in tokens array)
    let base_token_index = market_info.tokens[0];

    // Look up the token name using this index
    let token = spot_meta
        .tokens
        .get(base_token_index)
        .ok_or_else(|| anyhow!("Token index {} not found", base_token_index))?;

    Ok(token.name.clone())
}

/// Convert asset (string or number) to ticker name
pub async fn asset_to_ticker(asset: &str) -> Result<String> {
    // Check if it's an @ symbol
    if asset.starts_with('@') {
        return at_symbol_to_ticker(asset).await;
    }

    // Try parsing as number
    if let Ok(asset_num) = asset.parse::<u32>() {
        // Check if it's a spot index (>= 10000)
        if asset_num >= 10000 {
            if let Ok(ticker) = spot_index_to_ticker(asset_num).await {
                return Ok(ticker);
            }
        }

        // Try as perp index
        if let Ok(ticker) = perp_index_to_ticker(asset_num as usize).await {
            return Ok(ticker);
        }
    }

    // Return as-is if can't convert
    Ok(asset.to_string())
}

/// Convert ticker to asset (index or @ symbol)
pub async fn ticker_to_asset(ticker: &str, market_type: MarketType) -> Result<String> {
    // Check if already an @ symbol
    if ticker.starts_with('@') {
        return spot_ticker_to_at_symbol(ticker, None).await;
    }

    match market_type {
        MarketType::Spot => {
            let index = spot_ticker_to_index(ticker, None).await?;
            Ok(index.to_string())
        }
        MarketType::Perp => {
            let index = perp_ticker_to_index(ticker).await?;
            Ok(index.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_number() {
        assert_eq!(extract_number("@87").unwrap(), 87);
        assert_eq!(extract_number("@123").unwrap(), 123);
        assert!(extract_number("invalid").is_err());
    }
}
