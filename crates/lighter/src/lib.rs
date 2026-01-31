mod exchange_impl;
pub mod helpers;
pub mod types;
pub mod ws;

pub use exchange_impl::LighterExchange;
pub use ws::start_lighter_funding_feed;

pub const LIGHTER_WS_URL: &str = "wss://mainnet.zklighter.elliot.ai/stream";
pub const LIGHTER_HTTP_URL: &str = "https://mainnet.zklighter.elliot.ai";
pub const ORDER_TYPE_MARKET: u8 = 1;
