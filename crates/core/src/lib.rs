pub mod config;
pub mod exchange;
pub mod funding;
pub mod token_list;
pub mod types;
pub mod utils;
pub mod ws;

// Re-export commonly used types
pub use exchange::{
    AccountSettings, Exchange, ExchangeError, MarketInfo, PositionSide, UnifiedPosition, WsMessage,
};
pub use funding::{Dex, FundingSnapshot};
pub use types::LiveMarketFeed;
pub use utils::{parse_f64, parse_f64_or, parse_f64_or_zero};
pub use ws::{WsConfig, run_funding_feed};

// Solana program constants
pub const SOLANA_USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
pub const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
pub const ARBITRUM_USDC_ADDRESS: &str = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831";
pub const ARBITRUM_CHAIN_ID: u64 = 42161;
