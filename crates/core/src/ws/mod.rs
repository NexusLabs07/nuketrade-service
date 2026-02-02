//! Generic WebSocket infrastructure for exchange feeds.
//!
//! This module provides a unified WebSocket handler that works with any exchange
//! implementing the [`Exchange`] trait, eliminating code duplication across
//! exchange-specific implementations.

pub mod handler;
pub mod types;

pub use handler::run_funding_feed;
pub use types::WsConfig;
