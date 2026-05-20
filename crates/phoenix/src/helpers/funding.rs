pub const PLATFORM: &str = "phoenix";

/// Normalize legacy `funding_rate.rate` rows for Phoenix to hourly decimal (HL/Pacifica shape).
///
/// New writes already divide WS percent by 100 in `parse_ws_message`. Older rows may be:
/// - raw percent (e.g. `-0.0075` instead of `-0.000075`)
/// - percent incorrectly divided by 24 (e.g. `-0.000312`)
pub fn normalize_stored_hourly_rate(rate: f64) -> f64 {
    let abs = rate.abs();
    if abs > 0.002 {
        rate / 100.0
    } else if abs > 0.00025 {
        rate * 24.0 / 100.0
    } else {
        rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixes_percent_stored_legacy() {
        assert!((normalize_stored_hourly_rate(-0.007481) - (-0.00007481)).abs() < 1e-9);
    }

    #[test]
    fn fixes_div24_legacy() {
        assert!((normalize_stored_hourly_rate(-0.000312695) - (-0.000075047)).abs() < 1e-8);
    }

    #[test]
    fn leaves_correct_hourly_decimal() {
        assert!((normalize_stored_hourly_rate(-0.000075) - (-0.000075)).abs() < 1e-12);
    }
}
