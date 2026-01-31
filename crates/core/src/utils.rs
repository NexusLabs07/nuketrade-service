//! Utility functions for safe parsing and common operations.

/// Safely parse a string to f64, returning None on failure.
///
/// # Examples
/// ```
/// use core::utils::parse_f64;
/// assert_eq!(parse_f64("123.45"), Some(123.45));
/// assert_eq!(parse_f64("invalid"), None);
/// ```
#[inline]
pub fn parse_f64(s: &str) -> Option<f64> {
    s.parse().ok()
}

/// Safely parse a string to f64, returning 0.0 on failure.
///
/// This is useful when a default of 0.0 is acceptable for missing/invalid data.
///
/// # Examples
/// ```
/// use core::utils::parse_f64_or_zero;
/// assert_eq!(parse_f64_or_zero("123.45"), 123.45);
/// assert_eq!(parse_f64_or_zero("invalid"), 0.0);
/// ```
#[inline]
pub fn parse_f64_or_zero(s: &str) -> f64 {
    s.parse().unwrap_or(0.0)
}

/// Safely parse a string to f64 with a custom default.
///
/// # Examples
/// ```
/// use core::utils::parse_f64_or;
/// assert_eq!(parse_f64_or("123.45", -1.0), 123.45);
/// assert_eq!(parse_f64_or("invalid", -1.0), -1.0);
/// ```
#[inline]
pub fn parse_f64_or(s: &str, default: f64) -> f64 {
    s.parse().unwrap_or(default)
}

/// Safely parse a string to i64, returning None on failure.
#[inline]
pub fn parse_i64(s: &str) -> Option<i64> {
    s.parse().ok()
}

/// Safely parse a string to i64, returning 0 on failure.
#[inline]
pub fn parse_i64_or_zero(s: &str) -> i64 {
    s.parse().unwrap_or(0)
}

/// Safely parse a string to u32, returning None on failure.
#[inline]
pub fn parse_u32(s: &str) -> Option<u32> {
    s.parse().ok()
}

/// Safely parse a string to u32, returning 0 on failure.
#[inline]
pub fn parse_u32_or_zero(s: &str) -> u32 {
    s.parse().unwrap_or(0)
}

/// Truncate a float to a specified number of decimal places.
///
/// # Examples
/// ```
/// use core::utils::truncate_decimals;
/// assert_eq!(truncate_decimals(123.456789, 2), 123.45);
/// ```
#[inline]
pub fn truncate_decimals(value: f64, decimals: u32) -> f64 {
    let factor = 10_f64.powi(decimals as i32);
    (value * factor).trunc() / factor
}

/// Check if a string represents a positive number.
#[inline]
pub fn is_positive(s: &str) -> bool {
    parse_f64(s).map(|v| v > 0.0).unwrap_or(false)
}

/// Check if a string represents a negative number.
#[inline]
pub fn is_negative(s: &str) -> bool {
    parse_f64(s).map(|v| v < 0.0).unwrap_or(false)
}
