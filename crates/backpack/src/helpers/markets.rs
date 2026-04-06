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

#[cfg(test)]
mod tests {
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
}
