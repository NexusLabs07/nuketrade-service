//! Read-only RiseX integration.
//!
//! This crate intentionally contains no signing, order execution, account
//! mutation, transfer, deposit, withdrawal, or session-key functionality.

mod client;
mod feed;
mod types;

pub use client::{RISEX_HTTP_URL, RiseXClient};
pub use feed::start_risex_funding_feed;
pub use types::RiseXMarketSnapshot;
