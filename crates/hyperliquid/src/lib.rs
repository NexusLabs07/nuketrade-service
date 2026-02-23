pub mod exchange_impl;
pub mod helpers;
pub mod metadata;
pub mod ops;
pub mod ws;

pub use exchange_impl::HyperliquidExchange;
pub use ws::start_hl_funding_feed;

pub const HYPERLIQUID_WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
pub const HYPERLIQUID_HTTP_URL: &str = "https://api.hyperliquid.xyz";
pub const HYPERLIQUID_HTTP_TESTNET_URL: &str = "https://api.hyperliquid-testnet.xyz";
pub const HYPERLIQUID_DEPOSIT_CONTRACT_ADDRESS: &str = "0x377a90f0c3D1CfFc93815a5d4F6E705e047d6F04";
