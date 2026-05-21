pub mod apis;
pub mod exchange_impl;
pub mod helpers;
pub mod services;
pub mod ws;

pub use exchange_impl::PhoenixExchange;
pub use ws::start_phoenix_funding_feed;

pub const PHOENIX_HTTP_URL: &str = "https://perp-api.phoenix.trade";
pub const PHOENIX_WS_URL: &str = "wss://perp-api.phoenix.trade/v1/ws";
