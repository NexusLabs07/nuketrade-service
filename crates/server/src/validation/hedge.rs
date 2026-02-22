use perp_core::exchange::PerpetualExchange;
use validator::ValidationError;

pub fn validate_distinct_exchanges(
    exchanges: &[PerpetualExchange; 2],
) -> Result<(), ValidationError> {
    let [first, second] = exchanges;
    if first == second {
        let mut err = ValidationError::new("distinct_exchanges");
        err.message = Some("Exchanges must be different".into());
        return Err(err);
    }
    Ok(())
}
