mod types;
mod ws;

pub use ws::start_lighter_funding_feed;

pub const LIGHTER_WS_URL: &'static str = "wss://mainnet.zklighter.elliot.ai/stream";
