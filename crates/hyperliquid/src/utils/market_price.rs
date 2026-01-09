use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://api.hyperliquid.xyz";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub name: String,
    #[serde(rename = "szDecimals")]
    pub sz_decimals: u32,
    #[serde(rename = "isDelisted", default)]
    pub is_delisted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub px: String,
    pub sz: String,
    pub n: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2BookResponse {
    pub coin: String,
    pub time: u64,
    pub levels: Vec<Vec<OrderBookLevel>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaResponse {
    pub universe: Vec<AssetInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotToken {
    pub name: String,
    #[serde(rename = "tokenId")]
    pub token_id: String,
    #[serde(rename = "szDecimals")]
    pub sz_decimals: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotUniverse {
    pub name: String,
    pub tokens: [usize; 2],
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotMeta {
    pub tokens: Vec<SpotToken>,
    pub universe: Vec<SpotUniverse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDetailsResponse {
    #[serde(rename = "midPx")]
    pub mid_px: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpMeta {
    pub name: String,
    #[serde(rename = "szDecimals")]
    pub sz_decimals: u32,
}

#[derive(Debug, Clone)]
pub struct TickAndLotSize {
    pub sz_decimals: u32,
    pub px_decimals: u32,
}

impl TickAndLotSize {
    pub fn round_size(&self, size: f64) -> String {
        let factor = 10_f64.powi(self.sz_decimals as i32);
        let rounded_down_size = (size * factor).floor() / factor;

        // Remove trailing zeros by converting to f64 then to string
        let fixed = format!(
            "{:.prec$}",
            rounded_down_size,
            prec = self.sz_decimals as usize
        );
        fixed
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }

    pub fn round_price(&self, price: f64, asset_name: &str) -> String {
        // Round to 5 significant figures (max as per HL docs)
        let precision_decimals = if asset_name == "BTC" {
            let price_str = format!("{:.0}", price);
            if price_str.len() == 5 { 5 } else { 6 }
        } else {
            5
        };

        // Format to precision
        let precise_price_str = format!("{:.prec$e}", price, prec = precision_decimals - 1);
        let precise_price: f64 = precise_price_str.parse().unwrap_or(price);

        // Format to required decimal places
        let formatted_price = format!("{:.prec$}", precise_price, prec = self.px_decimals as usize);

        // Remove trailing zeros
        formatted_price
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

#[derive(Debug, Clone)]
pub struct AssetListItem {
    pub index: usize,
    pub name: String,
    pub price: String,
}

#[derive(Debug, Clone)]
pub struct MarketPrice {
    pub price: f64,
}

pub struct HyperliquidMarketPrice {
    client: reqwest::Client,
    base_url: String,
}

impl HyperliquidMarketPrice {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: BASE_URL.to_string(),
        }
    }

    pub fn with_base_url(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }

    /// Get current price for a perpetual asset by index
    pub async fn get_current_price(&self, asset_index: usize) -> Result<f64> {
        // First get asset info to get the coin name
        let meta_payload = serde_json::json!({
            "type": "meta"
        });

        let meta_response = self
            .client
            .post(&format!("{}/info", self.base_url))
            .json(&meta_payload)
            .send()
            .await?;

        let meta: MetaResponse = meta_response.json().await?;

        let asset = meta
            .universe
            .get(asset_index)
            .ok_or_else(|| anyhow!("Asset index {} not found", asset_index))?;

        if asset.is_delisted {
            return Err(anyhow!("Asset {} is delisted", asset.name));
        }

        // Get L2 book for the coin
        let book_payload = serde_json::json!({
            "type": "l2Book",
            "coin": asset.name
        });

        let book_response = self
            .client
            .post(&format!("{}/info", self.base_url))
            .json(&book_payload)
            .send()
            .await?;

        let book: L2BookResponse = book_response.json().await?;

        // Use the best bid price (levels[0][0].px)
        let price_str = book
            .levels
            .get(0)
            .and_then(|level| level.get(0))
            .map(|order| &order.px)
            .ok_or_else(|| anyhow!("No price data available for {}", asset.name))?;

        let price: f64 = price_str.parse()?;

        Ok(price)
    }

    /// Get current spot price for an asset
    pub async fn get_current_spot_price(&self, asset_name: &str) -> Result<f64> {
        // Map asset names
        let asset = match asset_name {
            "BTC" => "UBTC",
            "ETH" => "UETH",
            "SOL" => "USOL",
            _ => asset_name,
        };

        let spot_meta = self.get_spot_meta().await?;

        let token = spot_meta
            .tokens
            .iter()
            .find(|t| t.name == asset)
            .ok_or_else(|| anyhow!("Token {} not found in spot meta", asset))?;

        let token_id = &token.token_id;

        let payload = serde_json::json!({
            "type": "tokenDetails",
            "tokenId": token_id
        });

        let response = self
            .client
            .post(&format!("{}/info", self.base_url))
            .json(&payload)
            .send()
            .await?;

        let token_details: TokenDetailsResponse = response.json().await?;
        let mid_px: f64 = token_details.mid_px.parse()?;

        Ok(mid_px)
    }

    /// Get tick and lot size for an asset
    pub async fn get_tick_and_lot_size(
        &self,
        asset_name: &str,
        market: &str,
    ) -> Result<TickAndLotSize> {
        let max_decimals = if market == "perps" { 6 } else { 8 };

        // Map asset names for spot market
        let asset = if market == "spot" {
            match asset_name {
                "BTC" => "UBTC",
                "ETH" => "UETH",
                "SOL" => "USOL",
                _ => asset_name,
            }
        } else {
            asset_name
        };

        let sz_decimals = if market == "perps" {
            let perp_meta = self.get_perp_meta().await?;
            perp_meta
                .iter()
                .find(|t| t.name.to_uppercase() == asset.to_uppercase())
                .map(|t| t.sz_decimals)
                .ok_or_else(|| anyhow!("Asset {} not found in perp meta", asset))?
        } else {
            let spot_meta = self.get_spot_meta().await?;
            spot_meta
                .tokens
                .iter()
                .find(|t| t.name.to_uppercase() == asset.to_uppercase())
                .map(|t| t.sz_decimals)
                .ok_or_else(|| anyhow!("Asset {} not found in spot meta", asset))?
        };

        let px_decimals = max_decimals - sz_decimals;

        Ok(TickAndLotSize {
            sz_decimals,
            px_decimals,
        })
    }

    /// List all available (non-delisted) assets with their current prices
    pub async fn list_available_assets(&self) -> Result<Vec<AssetListItem>> {
        let meta_payload = serde_json::json!({
            "type": "meta"
        });

        let meta_response = self
            .client
            .post(&format!("{}/info", self.base_url))
            .json(&meta_payload)
            .send()
            .await?;

        let meta: MetaResponse = meta_response.json().await?;

        let mut assets = Vec::new();

        for (index, asset) in meta.universe.iter().enumerate() {
            if asset.is_delisted {
                continue;
            }

            match self.get_current_price(index).await {
                Ok(price) => {
                    assets.push(AssetListItem {
                        index,
                        name: asset.name.clone(),
                        price: price.to_string(),
                    });
                }
                Err(e) => {
                    eprintln!("Failed to get price for {}: {}", asset.name, e);
                }
            }
        }

        Ok(assets)
    }

    /// Check if a price is valid (within 80% deviation from current price)
    pub async fn is_price_valid(&self, asset_index: usize, order_price: f64) -> Result<bool> {
        let current_price = self.get_current_price(asset_index).await?;

        // Price can't deviate more than 80% up or down
        let upper_bound = current_price * 1.8; // +80%
        let lower_bound = current_price * 0.2; // -80%

        Ok(order_price >= lower_bound && order_price <= upper_bound)
    }

    /// Get market price for trading (GTC orders)
    /// For buy orders, returns best ask price
    /// For sell orders, returns best bid price
    pub async fn get_market_price_for_trading(
        &self,
        asset_ticker: &str,
        _market_type: &str, // spot or perps
        side: &str,         // buy or sell
    ) -> Result<MarketPrice> {
        // Fetch L2 book data for the asset ticker
        let payload = serde_json::json!({
            "type": "l2Book",
            "coin": asset_ticker.to_uppercase()
        });

        let response = self
            .client
            .post(&format!("{}/info", self.base_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch L2 book for {}: {}", asset_ticker, e))?;

        let book_data: L2BookResponse = response.json().await?;

        let price = if side == "buy" {
            // For buying, use the current best (lowest) ask price
            book_data
                .levels
                .get(1)
                .and_then(|level| level.get(0))
                .map(|order| order.px.parse::<f64>())
                .transpose()?
                .ok_or_else(|| {
                    anyhow!(
                        "No asks found in L2 book for {} to determine buy price for GTC limit",
                        asset_ticker
                    )
                })?
        } else {
            // For selling, use the current best (highest) bid price
            book_data
                .levels
                .get(0)
                .and_then(|level| level.get(0))
                .map(|order| order.px.parse::<f64>())
                .transpose()?
                .ok_or_else(|| {
                    anyhow!(
                        "No bids found in L2 book for {} to determine sell price for GTC limit",
                        asset_ticker
                    )
                })?
        };

        // COMMENT: If this function were part of a flow that also accepted an optional user-defined limit price,
        // the calling function (e.g., in TradeService) would make the decision:
        // - If a user-defined limitPrice is provided by the frontend for a GTC order,
        //   that user-defined price would be used INSTEAD of this fetched 'price'.
        // - This function's purpose is to determine an aggressive GTC price when no specific limit is given by the user.
        // - Example (in calling function):
        //   let finalPriceToUse = userSuppliedLimitPrice ? parseFloat(userSuppliedLimitPrice) : fetchedMarketPrice.price;
        //   Then, finalPriceToUse would be formatted and sent in the order.

        Ok(MarketPrice { price })
    }

    /// Get aggressive IOC (Immediate-Or-Cancel) price string
    /// This crosses the spread by one tick to ensure immediate execution
    pub async fn get_aggressive_ioc_price_string(
        &self,
        side: &str, // buy or sell
        asset_ticker: &str,
        market_type: &str, // perps or spot
    ) -> Result<String> {
        let tick_info = self
            .get_tick_and_lot_size(asset_ticker, market_type)
            .await?;

        // Fetch L2 book data for the asset ticker
        let payload = serde_json::json!({
            "type": "l2Book",
            "coin": asset_ticker.to_uppercase()
        });

        let response = self
            .client
            .post(&format!("{}/info", self.base_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                anyhow!(
                    "Could not fetch L2 book for {} to set IOC price: {}",
                    asset_ticker,
                    e
                )
            })?;

        let book_data: L2BookResponse = response.json().await?;

        let smallest_price_step = 10_f64.powi(-(tick_info.px_decimals as i32));

        let target_price = if side == "buy" {
            // For buying to close a short, use the current best (lowest) ask price,
            // then make it slightly more aggressive (one tick higher) to cross the spread
            let best_ask_price = book_data
                .levels
                .get(1)
                .and_then(|level| level.get(0))
                .map(|order| order.px.parse::<f64>())
                .transpose()?
                .ok_or_else(|| {
                    anyhow!(
                        "No asks found in L2 book for {} to place buy IOC",
                        asset_ticker
                    )
                })?;

            best_ask_price + smallest_price_step
        } else {
            // For selling to close a long, use the current best (highest) bid price,
            // then make it slightly more aggressive (one tick lower) to cross the spread
            let best_bid_price = book_data
                .levels
                .get(0)
                .and_then(|level| level.get(0))
                .map(|order| order.px.parse::<f64>())
                .transpose()?
                .ok_or_else(|| {
                    anyhow!(
                        "No bids found in L2 book for {} to place sell IOC",
                        asset_ticker
                    )
                })?;

            let potential_target_price = best_bid_price - smallest_price_step;

            // If subtracting a tick makes the price too low, use the smallest_price_step itself
            if potential_target_price < smallest_price_step {
                smallest_price_step
            } else {
                potential_target_price
            }
        };

        let final_price_string = tick_info.round_price(target_price, asset_ticker);

        Ok(final_price_string)
    }

    // Helper functions to fetch metadata
    async fn get_spot_meta(&self) -> Result<SpotMeta> {
        let payload = serde_json::json!({
            "type": "spotMeta"
        });

        let response = self
            .client
            .post(&format!("{}/info", self.base_url))
            .json(&payload)
            .send()
            .await?;

        let spot_meta: SpotMeta = response.json().await?;
        Ok(spot_meta)
    }

    async fn get_perp_meta(&self) -> Result<Vec<PerpMeta>> {
        let payload = serde_json::json!({
            "type": "meta"
        });

        let response = self
            .client
            .post(&format!("{}/info", self.base_url))
            .json(&payload)
            .send()
            .await?;

        let meta: MetaResponse = response.json().await?;

        // Convert AssetInfo to PerpMeta
        let perp_meta: Vec<PerpMeta> = meta
            .universe
            .into_iter()
            .map(|asset| PerpMeta {
                name: asset.name,
                sz_decimals: asset.sz_decimals,
            })
            .collect();

        Ok(perp_meta)
    }

    // Public versions for asset_index_converter
    pub async fn get_spot_meta_public(&self) -> Result<SpotMeta> {
        self.get_spot_meta().await
    }

    pub async fn get_perp_meta_public(&self) -> Result<Vec<PerpMeta>> {
        self.get_perp_meta().await
    }
}

impl Default for HyperliquidMarketPrice {
    fn default() -> Self {
        Self::new()
    }
}
