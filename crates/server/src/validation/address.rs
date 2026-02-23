use perp_core::chains::get_usdc_address;

use crate::features::bridge::types::QuotePayload;

pub fn validate_evm_address(address: &str) -> Result<(), validator::ValidationError> {
    if !address.starts_with("0x") {
        let mut err = validator::ValidationError::new("must_start_with_0x");
        err.message = Some("EVM address must start with 0x".into());
        return Err(err);
    }

    if address.len() != 42 {
        let mut err = validator::ValidationError::new("invalid_length");
        err.message = Some("EVM address must be exactly 42 characters".into());
        return Err(err);
    }

    let hex_part = &address[2..];
    if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        let mut err = validator::ValidationError::new("invalid_hex");
        err.message = Some("EVM address must contain only valid hex characters".into());
        return Err(err);
    }

    Ok(())
}

pub fn validate_solana_address(address: &str) -> Result<(), validator::ValidationError> {
    if address.is_empty() {
        let mut err = validator::ValidationError::new("empty_address");
        err.message = Some("Solana address must not be empty".into());
        return Err(err);
    }

    if !address.chars().all(|c| c.is_ascii_alphanumeric()) {
        let mut err = validator::ValidationError::new("invalid_base58");
        err.message = Some("Solana address must contain only alphanumeric characters".into());
        return Err(err);
    }

    if !(32..=44).contains(&address.len()) {
        let mut err = validator::ValidationError::new("invalid_length");
        err.message = Some("Solana address must be between 32 and 44 characters".into());
        return Err(err);
    }

    Ok(())
}

pub fn validate_timeframe(timeframe: &str) -> Result<(), validator::ValidationError> {
    match timeframe {
        "30m" | "1h" | "24h" | "7d" | "30d" => Ok(()),
        _ => {
            let mut err = validator::ValidationError::new("invalid_timeframe");
            err.message = Some("Timeframe must be one of: 30m, 1h, 24h, 7d, 30d".into());
            Err(err)
        }
    }
}

pub fn validate_destination_usdc_address(
    payload: &QuotePayload,
) -> Result<(), validator::ValidationError> {
    let usdc_address = get_usdc_address(payload.destination_chain_id);

    if usdc_address.is_none() {
        return Err(validator::ValidationError::new("invalid_chain_id"));
    }

    Ok(())
}
