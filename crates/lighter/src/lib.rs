mod apis;
mod client;
mod constants;
mod helpers;
mod types;
mod ws;

pub use ws::start_lighter_funding_feed;

pub const LIGHTER_WS_URL: &'static str = "wss://mainnet.zklighter.elliot.ai/stream";
pub const LIGHTER_HTTP_URL: &'static str = "https://mainnet.zklighter.elliot.ai";
pub const ORDER_TYPE_MARKET: u8 = 1;
