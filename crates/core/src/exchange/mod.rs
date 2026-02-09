//! Exchange trait abstraction for unified exchange operations.
//!
//! This module provides a common interface for all supported exchanges,
//! enabling polymorphic handling of exchange-specific operations.

mod error;
mod traits;
mod types;

pub use error::ExchangeError;
pub use traits::Exchange;
pub use types::{PerpetualExchange, exchange_to_chain};

pub use crate::types::{AccountSettings, MarketInfo, PositionSide, UnifiedPosition};
