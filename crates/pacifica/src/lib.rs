pub mod apis;
mod exchange_impl;
pub mod helpers;
pub mod services;
pub mod ws;

pub use exchange_impl::PacificaExchange;
pub use ws::start_pacifica_funding_feed;

pub const PACIFICA_HTTP_URL: &str = "https://api.pacifica.fi/api/v1";
pub const PACIFICA_WS_URL: &str = "wss://ws.pacifica.fi/ws";

pub const DEPOSIT_DISCRIMINATOR: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];

pub const PACIFICA_CENTRAL_STATE_ADDRESS: &str = "9Gdmhq4Gv1LnNMp7aiS1HSVd7pNnXNMsbuXALCQRmGjY";
pub const PACIFICA_VAULT_ADDRESS: &str = "72R843XwZxqWhsJceARQQTTbYtWy6Zw9et2YV4FpRHTa";
pub const PACIFICA_PROGRAM_ADDRESS: &str = "PCFA5iYgmqK6MqPhWNKg7Yv7auX7VZ4Cx7T1eJyrAMH";
pub const EVENT_AUTHORITY: &str = "2cPFdP7ADcdQE2rG9BqASYAVosZv3PX5yCyTdYCfGq8V";
