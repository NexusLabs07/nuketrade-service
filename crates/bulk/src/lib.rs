//! Read-only Bulk Trade adapter.
//!
//! This crate deliberately contains no signed transaction or custody logic.
//! It owns public market discovery, account reads, and the live ticker feed
//! used by the shared funding-rate pipeline.

mod exchange_impl;
mod types;
mod ws;

#[cfg(test)]
mod network_config_tests;

pub use exchange_impl::BulkExchange;
pub use types::{BulkFullAccount, BulkMarket};
pub use ws::start_bulk_funding_feed;

/// Bulk's dedicated mainnet HTTP API endpoint.
pub const BULK_MAINNET_HTTP_URL: &str = "https://mainnet-api1.bulk.trade/api/v1";

/// Bulk's dedicated mainnet market-data WebSocket endpoint.
pub const BULK_MAINNET_WS_URL: &str = "wss://mainnet-ws1.bulk.trade";

/// Bulk's testnet HTTP API endpoint.
pub const BULK_TESTNET_HTTP_URL: &str = "https://exchange-api.bulk.trade/api/v1";

/// Bulk's testnet market-data WebSocket endpoint.
pub const BULK_TESTNET_WS_URL: &str = "wss://exchange-ws1.bulk.trade";

/// The Bulk network selected for a process.
///
/// The network is intentionally kept beside endpoint and signing-domain
/// configuration so a caller cannot accidentally combine a mainnet endpoint
/// with a testnet signature domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkNetwork {
    Mainnet,
    Testnet,
}

impl BulkNetwork {
    /// Parses the supported deployment network from a human-readable value.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "mainnet" => Ok(Self::Mainnet),
            "testnet" => Ok(Self::Testnet),
            "devnet" => Err("Bulk devnet endpoints are not configured".to_string()),
            other => Err(format!(
                "unsupported BULK_NETWORK '{other}'; expected mainnet or testnet"
            )),
        }
    }

    /// Reads the required deployment network from the process environment.
    pub fn from_env() -> Result<Self, String> {
        let value = std::env::var("BULK_NETWORK")
            .map_err(|_| "BULK_NETWORK environment variable is required".to_string())?;
        Self::parse(&value)
    }

    /// Returns the stable wire value used in the internal feed contract.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mainnet => "mainnet",
            Self::Testnet => "testnet",
        }
    }

    /// Returns the complete endpoint and signing-domain configuration for this
    /// network.
    pub const fn config(self) -> BulkNetworkConfig {
        match self {
            Self::Mainnet => BulkNetworkConfig {
                network: Self::Mainnet,
                http_url: BULK_MAINNET_HTTP_URL,
                ws_url: BULK_MAINNET_WS_URL,
                signature_domain: 1,
            },
            Self::Testnet => BulkNetworkConfig {
                network: Self::Testnet,
                http_url: BULK_TESTNET_HTTP_URL,
                ws_url: BULK_TESTNET_WS_URL,
                signature_domain: 2,
            },
        }
    }
}

/// Immutable Bulk network configuration used by the read-only adapter and,
/// later, by the signed TypeScript execution plane's equivalent configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BulkNetworkConfig {
    pub network: BulkNetwork,
    pub http_url: &'static str,
    pub ws_url: &'static str,
    pub signature_domain: u8,
}

impl BulkNetworkConfig {
    /// Rejects endpoint or signature-domain combinations that do not belong to
    /// the selected Bulk network.
    pub fn validate(&self) -> Result<(), String> {
        let expected = self.network.config();

        if self.http_url != expected.http_url {
            return Err(format!(
                "Bulk {} HTTP endpoint does not match its network",
                self.network_name()
            ));
        }

        if self.ws_url != expected.ws_url {
            return Err(format!(
                "Bulk {} WebSocket endpoint does not match its network",
                self.network_name()
            ));
        }

        if self.signature_domain != expected.signature_domain {
            return Err(format!(
                "Bulk {} signature domain does not match its network",
                self.network_name()
            ));
        }

        Ok(())
    }

    fn network_name(&self) -> &'static str {
        match self.network {
            BulkNetwork::Mainnet => "mainnet",
            BulkNetwork::Testnet => "testnet",
        }
    }
}

// These aliases preserve compatibility for code that only consumes the
// read-only testnet adapter. Production startup must use `BulkNetwork::from_env`
// and the network-aware constructors below.
pub const BULK_HTTP_URL: &str = BULK_TESTNET_HTTP_URL;
pub const BULK_WS_URL: &str = BULK_TESTNET_WS_URL;
