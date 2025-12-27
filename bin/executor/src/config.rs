use anyhow::Context;

pub struct Config {
    pub db_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        let db_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL environment variable is not set or invalid")?;

        Ok(Self { db_url })
    }
}
