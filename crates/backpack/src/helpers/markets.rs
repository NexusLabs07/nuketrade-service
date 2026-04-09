use std::collections::HashSet;

use perp_core::parse_f64;

use crate::types::BackpackMarket;

/// Backpack public funding streams use exchange symbols like `SOL_USDC`.
/// Our backend uses symbols like `SOL`, so we normalize by taking the
/// base assest before the first underscore
pub fn canonical_symbol_from_backpack_symbol(symbol: &str) -> Option<String> {
    let base = symbol.split('_').next().unwrap_or(symbol).trim();

    if base.is_empty() {
        None
    } else {
        Some(base.to_string())
    }
}

pub fn is_allowed_backpack_symbol(symbol: &str, allowed_symbols: &HashSet<&str>) -> bool {
    canonical_symbol_from_backpack_symbol(symbol)
        .as_deref()
        .map(|canonical| allowed_symbols.contains(canonical))
        .unwrap_or(false)
}

/// we only subscribe to active PERP markets for now
pub fn is_active_perp_market(market: &BackpackMarket) -> bool {
    market.market_type.eq_ignore_ascii_case("PERP")
        && market.visible.unwrap_or(true)
        && !market
            .order_book_state
            .as_deref()
            .map(|state| state.eq_ignore_ascii_case("Closed"))
            .unwrap_or(false)
}

/// Inference from Backpack public docs:
/// public markets expose `imfFunction.base`, and official Backpack futures docs
/// define Base IMF in terms of platform/user max leverage. For a public feed,
/// the market-level platform max leverage is best represented as 1 / base IMF.
pub fn max_leverage_from_market(market: &BackpackMarket) -> Option<u32> {
    let base_imf = market
        .imf_function
        .as_ref()
        .and_then(|function| parse_f64(&function.base))?;

    if !base_imf.is_finite() || base_imf <= 0.0 {
        return None;
    }

    let leverage = (1.0 / base_imf).round();

    if !leverage.is_finite() || leverage < 1.0 {
        None
    } else {
        Some(leverage as u32)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::types::{BackpackMarket, MarginFunction};

    use super::*;

    #[test]
    fn canonical_symbol_parses_linear_perp_symbols() {
        assert_eq!(
            canonical_symbol_from_backpack_symbol("SOL_USDC").as_deref(),
            Some("SOL")
        );
        assert_eq!(
            canonical_symbol_from_backpack_symbol("BTC_USDC_PERP").as_deref(),
            Some("BTC")
        );
        assert_eq!(
            canonical_symbol_from_backpack_symbol("1000PEPE_USDC").as_deref(),
            Some("1000PEPE")
        );
    }

    #[test]
    fn allowlist_matches_against_canonical_symbol() {
        let allowed = HashSet::from(["SOL", "BTC"]);

        assert!(is_allowed_backpack_symbol("SOL_USDC", &allowed));
        assert!(is_allowed_backpack_symbol("BTC_USDC_PERP", &allowed));
        assert!(!is_allowed_backpack_symbol("AVNT_USDC", &allowed));
    }

    #[test]
    fn derives_max_leverage_from_imf_base() {
        let market = BackpackMarket {
            symbol: "ASTER_USDC".to_string(),
            base_symbol: "ASTER".to_string(),
            quote_symbol: "USDC".to_string(),
            market_type: "PERP".to_string(),
            order_book_state: Some("Open".to_string()),
            visible: Some(true),
            imf_function: Some(MarginFunction {
                function_type: "sqrt".to_string(),
                base: "0.2".to_string(),
                factor: "0.0001".to_string(),
            }),
            mmf_function: None,
        };

        assert_eq!(max_leverage_from_market(&market), Some(5));
    }
}
