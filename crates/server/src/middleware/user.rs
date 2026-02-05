use crate::features::user::controller::CreateUserPayload;

pub fn validate_addresses_are_null(
    payload: &CreateUserPayload,
) -> Result<(), validator::ValidationError> {
    if payload.connected_evm_address.is_some() {
        return Err(validator::ValidationError::new(
            "connected_evm_address must be null for now",
        ));
    }
    if payload.connected_solana_address.is_some() {
        return Err(validator::ValidationError::new(
            "connected_solana_address must be null for now",
        ));
    }
    if payload.turnkey_evm_address.is_some() {
        return Err(validator::ValidationError::new(
            "turnkey_evm_address must be null for now",
        ));
    }
    Ok(())
}

pub fn validate_evm_address(address: &str) -> Result<(), validator::ValidationError> {
    if !address.starts_with("0x") {
        return Err(validator::ValidationError::new("must_start_with_0x"));
    }

    if address.len() != 42 {
        return Err(validator::ValidationError::new("invalid_length"));
    }

    let hex_part = &address[2..];
    if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(validator::ValidationError::new("invalid_hex"));
    }

    Ok(())
}

pub fn validate_solana_address(address: &str) -> Result<(), validator::ValidationError> {
    if !address.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(validator::ValidationError::new("invalid_base58"));
    }

    Ok(())
}
