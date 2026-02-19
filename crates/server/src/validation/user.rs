use validator::ValidationError;

fn must_be_null_error(message: &'static str) -> ValidationError {
    let mut err = ValidationError::new("must_be_null");
    err.message = Some(message.into());
    err
}

// For Option<String> fields, validator runs on inner value (&String) when Some(...)
pub fn validate_connected_evm_address_none(_: &String) -> Result<(), ValidationError> {
    Err(must_be_null_error(
        "connected_evm_address must be null for now",
    ))
}

pub fn validate_connected_solana_address_none(_: &String) -> Result<(), ValidationError> {
    Err(must_be_null_error(
        "connected_solana_address must be null for now",
    ))
}

pub fn validate_turnkey_evm_address_none(_: &String) -> Result<(), ValidationError> {
    Err(must_be_null_error(
        "turnkey_evm_address must be null for now",
    ))
}
