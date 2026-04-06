//! Exchange enum and related type definitions.

use core::fmt;
use serde::{Deserialize, Serialize};

use crate::chains::{AddressType, Chain};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PerpetualExchange {
    Backpack,
    Hyperliquid,
    Lighter,
    Pacifica,
}

impl PerpetualExchange {
    /// All exchanges that participate in hedging (bridge + deposit flows).
    pub const HEDGEABLE: &'static [PerpetualExchange] =
        &[PerpetualExchange::Hyperliquid, PerpetualExchange::Pacifica];

    pub fn as_str(&self) -> &'static str {
        match self {
            PerpetualExchange::Backpack => "backpack",
            PerpetualExchange::Hyperliquid => "hyperliquid",
            PerpetualExchange::Lighter => "lighter",
            PerpetualExchange::Pacifica => "pacifica",
        }
    }

    /// The destination chain for this exchange (None for exchanges without a chain mapping).
    pub fn chain(&self) -> Option<Chain> {
        match self {
            PerpetualExchange::Backpack => None,
            PerpetualExchange::Hyperliquid => Some(Chain::ARBITRUM),
            PerpetualExchange::Pacifica => Some(Chain::SOLANA),
            PerpetualExchange::Lighter => None,
        }
    }

    /// Chain ID, or 0 if unknown.
    pub fn chain_id(&self) -> u64 {
        self.chain().map(|c| c.id).unwrap_or(0)
    }

    /// The bridge action name (e.g. "BRIDGE_BASE_TO_ARB").
    pub fn bridge_action(&self) -> Option<&'static str> {
        match self {
            PerpetualExchange::Backpack => None,
            PerpetualExchange::Hyperliquid => Some("BRIDGE_BASE_TO_ARB"),
            PerpetualExchange::Pacifica => Some("BRIDGE_BASE_TO_SOL"),
            PerpetualExchange::Lighter => None,
        }
    }

    /// The deposit action name (e.g. "DEPOSIT_TO_HYPERLIQUID").
    pub fn deposit_action(&self) -> Option<&'static str> {
        match self {
            PerpetualExchange::Backpack => None,
            PerpetualExchange::Hyperliquid => Some("DEPOSIT_TO_HYPERLIQUID"),
            PerpetualExchange::Pacifica => Some("DEPOSIT_TO_PACIFICA"),
            PerpetualExchange::Lighter => None,
        }
    }

    /// Which address family this exchange uses.
    pub fn address_type(&self) -> AddressType {
        match self {
            PerpetualExchange::Hyperliquid | PerpetualExchange::Lighter => AddressType::Evm,
            PerpetualExchange::Pacifica | PerpetualExchange::Backpack => AddressType::Solana,
        }
    }

    /// Pick the right user address for this exchange.
    pub fn resolve_address<'a>(&self, evm_address: &'a str, solana_address: &'a str) -> &'a str {
        match self.address_type() {
            AddressType::Evm => evm_address,
            AddressType::Solana => solana_address,
        }
    }

    /// Look up an exchange by its bridge action string.
    pub fn from_bridge_action(action: &str) -> Option<PerpetualExchange> {
        Self::HEDGEABLE
            .iter()
            .find(|e| e.bridge_action() == Some(action))
            .cloned()
    }

    /// Look up an exchange by its deposit action string.
    pub fn from_deposit_action(action: &str) -> Option<PerpetualExchange> {
        Self::HEDGEABLE
            .iter()
            .find(|e| e.deposit_action() == Some(action))
            .cloned()
    }

    /// Check if the given action string is a bridge action for any exchange.
    pub fn is_bridge_action(action: &str) -> bool {
        Self::from_bridge_action(action).is_some()
    }

    /// Check if the given action string is a deposit action for any exchange.
    pub fn is_deposit_action(action: &str) -> bool {
        Self::from_deposit_action(action).is_some()
    }
}

impl fmt::Display for PerpetualExchange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for PerpetualExchange {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hyperliquid" => Ok(PerpetualExchange::Hyperliquid),
            "lighter" => Ok(PerpetualExchange::Lighter),
            "pacifica" => Ok(PerpetualExchange::Pacifica),
            "backpack" => Ok(PerpetualExchange::Backpack),
            _ => Err(format!("Unknown exchange: {s}")),
        }
    }
}

/// Map a protocol to its chain ID.
pub fn exchange_to_chain(exchange: &PerpetualExchange) -> u64 {
    exchange.chain_id()
}
