pub mod apis;
mod exchange_impl;
pub mod helpers;
pub mod perp_metadata;
pub mod services;
pub mod spot_metadata;
pub mod types;
pub mod ws;

pub use exchange_impl::HyperliquidExchange;
pub use ws::start_hl_funding_feed;

pub const HYPERLIQUID_WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
pub const HYPERLIQUID_HTTP_URL: &str = "https://api.hyperliquid.xyz";
pub const HYPERLIQUID_HTTP_TESTNET_URL: &str = "https://api.hyperliquid-testnet.xyz";
