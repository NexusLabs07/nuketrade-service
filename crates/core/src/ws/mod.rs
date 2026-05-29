//! Generic WebSocket infrastructure for exchange feeds.
//!
//! This module provides a unified WebSocket handler that works with any exchange
//! implementing the [`Exchange`] trait, eliminating code duplication across
//! exchange-specific implementations.

pub mod config;
pub mod handler;

pub use config::{WsConfig, WsHeartbeat};
pub use handler::run_funding_feed;

/// WebSocket message types for funding feeds.
#[derive(Debug, Clone)]
pub enum WsMessage {
    /// Funding rate update with market data
    FundingUpdate {
        symbol: String,
        mark_price: f64,
        funding_rate: f64,
        timestamp_ms: i64,
    },
    /// Ping message for keepalive
    Ping,
    /// Pong response
    Pong,
    /// Connection closed
    Close,
    /// Unrecognized message
    Unknown(String),
}
