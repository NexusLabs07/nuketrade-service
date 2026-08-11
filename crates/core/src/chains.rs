#[derive(Debug, Clone, Copy)]
pub struct Chain {
    pub id: u64,
    pub name: &'static str,
    pub usdc_address: &'static str,
}

impl Chain {
    pub const ARBITRUM: Chain = Chain {
        id: 42161,
        name: "Arbitrum",
        usdc_address: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
    };

    pub const BASE: Chain = Chain {
        id: 8453,
        name: "Base",
        usdc_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
    };

    pub const ETHEREUM: Chain = Chain {
        id: 1,
        name: "Ethereum",
        usdc_address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    };

    pub const SOLANA: Chain = Chain {
        id: 792703809,
        name: "Solana",
        usdc_address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    };

    /// RISE mainnet. The collateral address is the USDC quote token currently
    /// reported by the production RiseX market configuration.
    pub const RISE: Chain = Chain {
        id: 4153,
        name: "RISE",
        usdc_address: "0xe436820ba0c69702c1d3e601d421c0ef38262739",
    };

    /// All supported chains for iteration
    pub const ALL: &'static [Chain] = &[
        Chain::ARBITRUM,
        Chain::BASE,
        Chain::ETHEREUM,
        Chain::SOLANA,
        Chain::RISE,
    ];

    /// Find a chain by its ID
    pub fn from_id(chain_id: u64) -> Option<&'static Chain> {
        Chain::ALL.iter().find(|c| c.id == chain_id)
    }
}

pub fn get_usdc_address(chain_id: u64) -> Option<&'static str> {
    Chain::from_id(chain_id).map(|c| c.usdc_address)
}

/// Which address family an exchange uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressType {
    Evm,
    Solana,
}

// Solana program constants
pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
pub const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
