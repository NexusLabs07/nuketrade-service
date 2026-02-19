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
