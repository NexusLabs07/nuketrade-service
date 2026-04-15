pub fn canonical_symbol_from_bulk_symbol(symbol: &str) -> Option<String> {
    let base = symbol.split('-').next().unwrap_or(symbol).trim();

    if base.is_empty() {
        None
    } else {
        Some(base.to_string())
    }
}

pub fn normalize_timestamp_ms(timestamp: i64) -> i64 {
    let magnitude = timestamp.unsigned_abs();

    if magnitude >= 1_000_000_000_000_000_000 {
        timestamp / 1_000_000
    } else if magnitude >= 1_000_000_000_000_000 {
        timestamp / 1_000
    } else if magnitude >= 1_000_000_000_000 {
        timestamp
    } else if magnitude >= 1_000_000_000 {
        timestamp * 1000
    } else {
        timestamp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_symbol_parses_dash_pairs() {
        assert_eq!(
            canonical_symbol_from_bulk_symbol("SOL-USD").as_deref(),
            Some("SOL")
        );
        assert_eq!(
            canonical_symbol_from_bulk_symbol("1000PEPE-USD").as_deref(),
            Some("1000PEPE")
        );
    }

    #[test]
    fn normalize_timestamp_converts_nanoseconds_to_millis() {
        assert_eq!(normalize_timestamp_ms(1763316177219383423), 1763316177219);
    }

    #[test]
    fn normalize_timestamp_keeps_millis_as_is() {
        assert_eq!(normalize_timestamp_ms(1763316177219), 1763316177219);
    }
}
