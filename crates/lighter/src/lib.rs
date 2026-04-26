pub mod apis;
mod exchange_impl;
pub mod helpers;
pub mod services;
pub mod types;
pub mod ws;

pub use exchange_impl::LighterExchange;
pub use ws::start_lighter_funding_feed;

pub const LIGHTER_WS_URL: &str = "wss://mainnet.zklighter.elliot.ai/stream?readonly=true";
pub const LIGHTER_HTTP_URL: &str = "https://mainnet.zklighter.elliot.ai";
pub const ORDER_TYPE_MARKET: u8 = 1;

/// Lighter's Ethereum mainnet L1 bridge / deposit gateway contract.
/// Accepts USDC (perp margin) via `deposit(deposit, _to, _assetIndex, _routeType, _amount)`.
pub const LIGHTER_DEPOSIT_CONTRACT_ADDRESS: &str = "0x3B4D794a66304F130a4Db8F2551B0070dfCf5ca7";

/// Route type for Lighter's perp (cross-margin) account.
pub const LIGHTER_ROUTE_TYPE_PERP: u64 = 0;

/// Route type for Lighter's spot account.
pub const LIGHTER_ROUTE_TYPE_SPOT: u64 = 1;

/// USDC asset index on Lighter's bridge contract. Lighter's docs say to pull
/// this from the `/api/v1/assetDetails` endpoint; USDC is currently asset 3.
pub const LIGHTER_USDC_ASSET_INDEX: u64 = 3;
