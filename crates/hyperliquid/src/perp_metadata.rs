//! Hyperliquid perpetual market metadata.
//!
//! This module provides static metadata for all perpetual markets on Hyperliquid.
//! The data includes market parameters like size decimals, max leverage, and margin table IDs.

/// Hyperliquid perpetual market metadata as a JSON string.
///
/// This data is loaded at compile time from the data file.
pub const PERP_META: &str = include_str!("../data/perp_metadata.json");
