mod exchange_impl;
pub mod helpers;
pub mod types;
pub mod ws;

pub use exchange_impl::BulkExchange;
pub use ws::start_bulk_funding_feed;

pub const BULK_HTTP_URL: &str = "https://exchange-api.bulk.trade/api/v1";
pub const BULK_WS_URL: &str = "wss://exchange-ws1.bulk.trade";
