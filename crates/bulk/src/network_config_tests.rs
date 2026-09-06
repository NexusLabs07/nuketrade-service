use super::{
    BULK_MAINNET_HTTP_URL, BULK_MAINNET_WS_URL, BULK_TESTNET_HTTP_URL, BULK_TESTNET_WS_URL,
    BulkNetwork, BulkNetworkConfig,
};

#[test]
fn mainnet_configuration_uses_mainnet_endpoints_and_domain() {
    let config = BulkNetwork::Mainnet.config();

    assert_eq!(config.http_url, BULK_MAINNET_HTTP_URL);
    assert_eq!(config.ws_url, BULK_MAINNET_WS_URL);
    assert_eq!(config.signature_domain, 1);
}

#[test]
fn testnet_configuration_uses_testnet_endpoints_and_domain() {
    let config = BulkNetwork::Testnet.config();

    assert_eq!(config.http_url, BULK_TESTNET_HTTP_URL);
    assert_eq!(config.ws_url, BULK_TESTNET_WS_URL);
    assert_eq!(config.signature_domain, 2);
}

#[test]
fn mismatched_endpoint_and_network_configuration_is_rejected() {
    let config = BulkNetworkConfig {
        network: BulkNetwork::Mainnet,
        http_url: BULK_TESTNET_HTTP_URL,
        ws_url: BULK_TESTNET_WS_URL,
        signature_domain: 2,
    };

    assert!(config.validate().is_err());
}
