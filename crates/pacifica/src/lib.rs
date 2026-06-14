pub mod apis;
mod exchange_impl;
pub mod helpers;
pub mod perp_metadata;
pub mod services;
pub mod ws;

pub use exchange_impl::PacificaExchange;
pub use ws::start_pacifica_funding_feed;

pub const PACIFICA_HTTP_URL: &str = "https://api.pacifica.fi/api/v1";
pub const PACIFICA_WS_URL: &str = "wss://ws.pacifica.fi/ws";
