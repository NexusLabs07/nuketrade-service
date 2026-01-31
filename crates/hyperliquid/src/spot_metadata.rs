//! Hyperliquid spot market metadata.
//!
//! This module provides static metadata for all spot markets on Hyperliquid.
//! The data includes token information, market parameters, and trading pairs.

/// Hyperliquid spot market metadata as a JSON string.
///
/// This data is loaded at compile time from the data file.
pub const SPOT_META: &str = include_str!("../data/spot_metadata.json");
