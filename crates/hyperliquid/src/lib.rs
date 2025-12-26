mod types;
mod ws;

pub use ws::start_hl_funding_feed;

pub const HYPERLIQUID_WS_URL: &'static str = "wss://api.hyperliquid.com/v1/ws";