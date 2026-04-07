pub mod exchange_impl;
pub mod helpers;
pub mod types;
pub mod ws;

pub use exchange_impl::BackpackExchange;
pub use ws::start_backpack_funding_feed;

pub const BACKPACK_WS_URL: &str = "wss://ws.backpack.exchange";
pub const BACKPACK_HTTP_URL: &str = "https://api.backpack.exchange/api/v1";
