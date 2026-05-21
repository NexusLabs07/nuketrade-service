pub fn normalize_phoenix_symbol(symbol: &str) -> String {
    symbol
        .trim()
        .trim_end_matches("-PERP")
        .trim_end_matches("/USD")
        .to_ascii_uppercase()
}
