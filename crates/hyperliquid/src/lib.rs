mod apis;
mod types;
mod utils;
mod ws;

pub use apis::perp::{
    CancelOrderRequest, PerpOrderRequest, cancel_order_typed_data,
    close_all_perp_position_typed_data, close_perp_position_typed_data,
    create_perp_position_typed_data,
};
pub use apis::tp_sl::{CancelTpSlParams, TpSlManager, TpSlParams, TpSlResponse, UpdateTpSlParams};
pub use utils::asset_index_converter::{
    MarketType, asset_to_ticker, at_symbol_to_ticker, perp_index_to_ticker, perp_ticker_to_index,
    spot_index_to_ticker, spot_ticker_to_at_symbol, spot_ticker_to_index, ticker_to_all_mids_key,
    ticker_to_asset,
};
pub use utils::market_price::{
    AssetInfo, AssetListItem, HyperliquidMarketPrice, L2BookResponse, MarketPrice, OrderBookLevel,
    PerpMeta, SpotMeta, SpotToken, SpotUniverse, TickAndLotSize,
};
pub use ws::start_hl_funding_feed;

pub const HYPERLIQUID_WS_URL: &'static str = "wss://api.hyperliquid.xyz/ws";
pub const HYPERLIQUID_HTTP_URL: &'static str = "https://api.hyperliquid.xyz";
pub const HYPERLIQUID_HTTP_TESTNET_URL: &'static str = "https://api.hyperliquid-testnet.xyz";
