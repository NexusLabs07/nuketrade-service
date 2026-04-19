use anyhow::Context;

/// Application configuration loaded from environment variables.
///
/// Required variables:
/// - `DATABASE_URL`: PostgreSQL connection URL
/// - `SOLANA_RPC_URL`: Solana RPC endpoint
/// - `SERVER_HOST`: Server bind host (default: 0.0.0.0)
/// - `SERVER_PORT`: Server bind port (default: 8000)
/// - `CORS_ALLOWED_ORIGINS`: Comma-separated CORS origins
/// - `FEE_PAYER_PRIVATE_KEY`: Fee payer private key
#[derive(Debug, Clone)]
pub struct Config {
    // Database
    pub db_url: String,

    // Solana
    pub solana_rpc_url: String,

    // Arbitrum
    pub arbitrum_rpc_url: String,

    // Base
    pub base_rpc_url: String,

    // Ethereum
    pub ethereum_rpc_url: String,

    // Server
    pub server_host: String,
    pub server_port: u16,
    pub cors_allowed_origins: Vec<String>,

    //Private keys
    pub evm_fee_payer_private_key: String,
    pub solana_fee_payer_private_key: String,

    //Relay
    pub relay_api_key: String,

    // Google OAuth
    pub google_client_id: String,

    // Auth / Turnkey
    pub turnkey_api_base_url: String,
    pub turnkey_parent_org_id: String,
    pub turnkey_api_public_key: String,
    pub turnkey_api_private_key: String,
    pub auth_jwt_secret: String,
    pub auth_jwt_ttl_days: u64,

    // Optional access code gate for login
    pub access_code: Option<String>,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// # Errors
    /// Returns an error if required environment variables are missing.
    pub fn from_env() -> Result<Self, anyhow::Error> {
        // Required variables
        let db_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL environment variable is required")?;

        let solana_rpc_url = std::env::var("SOLANA_RPC_URL")
            .context("SOLANA_RPC_URL environment variable is required")?;

        // Arbitrum config
        let arbitrum_rpc_url = std::env::var("ARBITRUM_RPC_URL")
            .unwrap_or_else(|_| "https://arb1.arbitrum.io/rpc".to_string());

        let base_rpc_url =
            std::env::var("BASE_RPC_URL").unwrap_or_else(|_| "https://1rpc.io/base".to_string());

        let ethereum_rpc_url = std::env::var("ETHEREUM_RPC_URL")
            .unwrap_or_else(|_| "https://eth.llamarpc.com".to_string());

        // Server config
        let server_host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let server_port = std::env::var("SERVER_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8000);
        let cors_allowed_origins = std::env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "https://nuketrade.xyz,http://localhost:3000".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        let evm_fee_payer_private_key = std::env::var("EVM_FEE_PAYER_PRIVATE_KEY")
            .context("EVM_FEE_PAYER_PRIVATE_KEY is required")?;

        let solana_fee_payer_private_key = std::env::var("SOLANA_FEE_PAYER_PRIVATE_KEY")
            .context("SOLANA_FEE_PAYER_PRIVATE_KEY is required")?;

        let relay_api_key = std::env::var("RELAY_API_KEY").context("RELAY API KEY is required")?;

        let turnkey_api_base_url = std::env::var("TURNKEY_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.turnkey.com".to_string());

        let turnkey_parent_org_id = std::env::var("TURNKEY_PARENT_ORG_ID")
            .or_else(|_| std::env::var("TURNKEY_ORGANIZATION_ID"))
            .context("TURNKEY_PARENT_ORG_ID (or TURNKEY_ORGANIZATION_ID) is required")?;

        let turnkey_api_public_key = std::env::var("TURNKEY_API_PUBLIC_KEY")
            .context("TURNKEY_API_PUBLIC_KEY is required")?;

        let turnkey_api_private_key = std::env::var("TURNKEY_API_PRIVATE_KEY")
            .context("TURNKEY_API_PRIVATE_KEY is required")?;

        let auth_jwt_secret = std::env::var("AUTH_JWT_SECRET")
            .or_else(|_| std::env::var("JWT_SECRET"))
            .context("AUTH_JWT_SECRET (or JWT_SECRET) is required")?;

        let auth_jwt_ttl_days = std::env::var("AUTH_JWT_TTL_DAYS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(15);

        let google_client_id = std::env::var("GOOGLE_CLIENT_ID")
            .context("GOOGLE_CLIENT_ID environment variable is required")?;

        let access_code = std::env::var("ACCESS_CODE").ok();

        Ok(Self {
            db_url,
            evm_fee_payer_private_key,
            solana_fee_payer_private_key,
            relay_api_key,
            solana_rpc_url,
            arbitrum_rpc_url,
            base_rpc_url,
            ethereum_rpc_url,
            server_host,
            server_port,
            cors_allowed_origins,
            turnkey_api_base_url,
            turnkey_parent_org_id,
            turnkey_api_public_key,
            turnkey_api_private_key,
            auth_jwt_secret,
            auth_jwt_ttl_days,
            google_client_id,
            access_code,
        })
    }

    /// Get the server bind address as a string.
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }
}
