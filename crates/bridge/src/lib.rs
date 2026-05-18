pub mod client;
pub mod sponsor;

pub const RELAY_API_URL: &str = "https://api.relay.link";

pub const SUPPORTED_CHAINS: [u64; 5] = [1, 8453, 42161, 1337, 792703809];

pub const MIN_BRIDGE_AMOUNT: u64 = 2_000_000; //2 usdc
