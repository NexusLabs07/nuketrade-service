//! Pacifica perpetual market metadata.
//!
//! This module provides static metadata for all perpetual markets on Pacifica.
//! The data includes market parameters like tick size, lot size, max leverage, and funding rates.

/// Pacifica perpetual market metadata as a JSON string.
///
/// This data is loaded at compile time from the data file.
pub const PERP_META: &str = include_str!("../data/perp_metadata.json");
