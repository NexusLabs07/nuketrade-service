//! Read-only Bulk Trade adapter.
//!
//! This crate deliberately contains no signed transaction or custody logic.
//! It owns public market discovery, account reads, and the live ticker feed
//! used by the shared funding-rate pipeline.

mod exchange_impl;
mod types;
mod ws;

pub use exchange_impl::BulkExchange;
pub use types::{BulkFullAccount, BulkMarket};
pub use ws::start_bulk_funding_feed;

pub const BULK_HTTP_URL: &str = "https://exchange-api.bulk.trade/api/v1";
pub const BULK_WS_URL: &str = "wss://exchange-ws1.bulk.trade";
