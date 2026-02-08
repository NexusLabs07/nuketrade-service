//! Account settings types.

use serde::{Deserialize, Serialize};

/// Account settings returned from exchanges.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountSettings {
    pub leverage: u32,
    pub margin_mode: Option<String>,
    pub collateral: f64,
}
