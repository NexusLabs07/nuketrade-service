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

    pub const SOLANA: Chain = Chain {
        id: 792703809,
        name: "Solana",
        usdc_address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    };

    /// All supported chains for iteration
    pub const ALL: &'static [Chain] = &[Chain::ARBITRUM, Chain::BASE, Chain::SOLANA];

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