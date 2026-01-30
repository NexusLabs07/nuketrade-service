use anyhow::Context;

pub struct Config {
    pub db_url: String,
    pub solana_rpc_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        let db_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL environment variable is not set or invalid")?;

        let solana_rpc_url = std::env::var("SOLANA_RPC_URL")
            .context("SOLANA_RPC_URL environment variable is not set or invalid")?;

        Ok(Self {
            db_url,
            solana_rpc_url,
        })
    }
}
