use thiserror::Error;

pub struct Config {
    pub db_url: String,
}

#[derive(Error, Debug)]
enum ConfigError {
    #[error("Invalid DB URL")]
    InvalidDbUrl,
}

impl Config {
    pub fn get_config() -> Self {
        let db_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| ConfigError::InvalidDbUrl.to_string());

        Self { db_url }
    }
}
