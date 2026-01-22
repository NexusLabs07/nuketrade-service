pub mod apis;
pub mod perp_metadata;
pub mod spot_metadata;
mod types;
mod ws;
pub use ws::start_hl_funding_feed;

pub const HYPERLIQUID_WS_URL: &'static str = "wss://api.hyperliquid.xyz/ws";
pub const HYPERLIQUID_HTTP_URL: &'static str = "https://api.hyperliquid.xyz";
pub const HYPERLIQUID_HTTP_TESTNET_URL: &'static str = "https://api.hyperliquid-testnet.xyz";
