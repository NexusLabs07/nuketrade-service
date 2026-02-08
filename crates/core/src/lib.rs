pub mod chains;
pub mod config;
pub mod exchange;
pub mod token_list;
pub mod types;
pub mod utils;
pub mod ws;

// Re-export commonly used types
pub use exchange::{
    AccountSettings, Exchange, ExchangeError, MarketInfo, PositionSide, UnifiedPosition, WsMessage,
};
pub use types::LiveMarketFeed;
pub use utils::{parse_f64, parse_f64_or, parse_f64_or_zero};
pub use ws::{WsConfig, run_funding_feed};

// Solana program constants
pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
pub const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";

pub use chains::Chain;
