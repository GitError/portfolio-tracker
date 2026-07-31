use crate::types::FxRate;

/// Convert `amount` from `from_currency` into `base` using the cached `rates`.
///
/// Returns `Some(converted)` when a rate is available, or `None` when no
/// matching rate exists in the cache. Callers should treat `None` as a signal
/// that the conversion is unreliable and surface a stale-FX warning to the
/// user rather than silently falling back to a 1:1 rate.
pub fn convert_to_base(amount: f64, from_currency: &str, base: &str, rates: &[FxRate]) -> Option<f64> {
    let from_upper = from_currency.to_uppercase();
    let base_upper = base.to_uppercase();
    if from_upper == base_upper {
        return Some(amount);
    }

    // Try the direct pair first: e.g. USDCAD when converting USD → CAD
    let direct_pair = format!("{}{}", from_upper, base_upper);
    if let Some(rate) = rates.iter().find(|r| r.pair == direct_pair) {
        if !rate.rate.is_finite() || rate.rate == 0.0 {
            tracing::warn!(pair = %rate.pair, rate = rate.rate, "FX rate is zero or non-finite; returning amount with fxStale=true");
            return None;
        }
        return Some(amount * rate.rate);
    }

    // Fall back to the inverted pair: e.g. CADUSD when converting USD → CAD
    // but base=CAD was previously cached as USDCAD. Invert the stored rate.
    let inverted_pair = format!("{}{}", base_upper, from_upper);
    if let Some(rate) = rates.iter().find(|r| r.pair == inverted_pair) {
        if rate.rate.is_finite() && rate.rate != 0.0 {
            return Some(amount / rate.rate);
        }
    }

    tracing::warn!(
        "FX rate not found for {} → {}, holding will be marked as fx_stale",
        from_currency,
        base
    );
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rate(pair: &str, rate: f64) -> FxRate {
        FxRate {
            pair: pair.to_string(),
            rate,
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn base_passthrough_returns_amount_unchanged() {
        let rates = vec![make_rate("USDCAD", 1.36)];
        assert_eq!(convert_to_base(100.0, "CAD", "CAD", &rates), Some(100.0));
        assert_eq!(convert_to_base(100.0, "cad", "CAD", &rates), Some(100.0));
        assert_eq!(convert_to_base(100.0, "USD", "USD", &rates), Some(100.0));
    }

    #[test]
    fn usd_converts_to_cad_correctly() {
        let rates = vec![make_rate("USDCAD", 1.36)];
        let result = convert_to_base(100.0, "USD", "CAD", &rates).unwrap();
        assert!((result - 136.0).abs() < 0.001);
    }

    #[test]
    fn cad_converts_to_usd_correctly() {
        let rates = vec![make_rate("CADUSD", 0.735)];
        let result = convert_to_base(100.0, "CAD", "USD", &rates).unwrap();
        assert!((result - 73.5).abs() < 0.001);
    }

    #[test]
    fn missing_rate_returns_none() {
        let result = convert_to_base(200.0, "EUR", "CAD", &[]);
        assert_eq!(result, None);
    }

    #[test]
    fn cad_converts_to_usd_using_inverted_usdcad_pair() {
        // Only USDCAD is cached (as stored when CAD was the base). When base switches
        // to USD we must invert the stored rate rather than return unconverted.
        let rates = vec![make_rate("USDCAD", 1.36)];
        let result = convert_to_base(100.0, "CAD", "USD", &rates).unwrap();
        // 100 CAD / 1.36 ≈ 73.529
        assert!((result - (100.0_f64 / 1.36)).abs() < 0.001);
    }

    #[test]
    fn direct_pair_zero_rate_returns_none() {
        // A cached direct pair with rate == 0.0 must return None (fx_stale), not 0.
        let rates = vec![make_rate("USDCAD", 0.0)];
        let result = convert_to_base(100.0, "USD", "CAD", &rates);
        assert_eq!(
            result, None,
            "zero direct rate should be treated as unavailable, not a valid 0 conversion"
        );
    }

    #[test]
    fn direct_pair_infinite_rate_returns_none() {
        // Regression guard for #638: an infinite direct rate must be rejected too.
        let rates = vec![make_rate("USDCAD", f64::INFINITY)];
        let result = convert_to_base(100.0, "USD", "CAD", &rates);
        assert_eq!(result, None);
    }

    #[test]
    fn direct_pair_nan_rate_returns_none() {
        let rates = vec![make_rate("USDCAD", f64::NAN)];
        let result = convert_to_base(100.0, "USD", "CAD", &rates);
        assert_eq!(result, None);
    }

    #[test]
    fn inverted_pair_infinite_rate_returns_none() {
        let rates = vec![make_rate("CADUSD", f64::INFINITY)];
        let result = convert_to_base(100.0, "USD", "CAD", &rates);
        assert_eq!(result, None);
    }

    #[test]
    fn inverted_pair_zero_rate_returns_none() {
        // An inverted pair with rate == 0.0 would divide by zero — must return None.
        let rates = vec![make_rate("CADUSD", 0.0)];
        let result = convert_to_base(100.0, "USD", "CAD", &rates);
        assert_eq!(result, None);
    }

    #[test]
    fn unknown_currency_pair_returns_none() {
        let rates = vec![make_rate("USDCAD", 1.36)];
        assert_eq!(convert_to_base(100.0, "GBP", "CAD", &rates), None);
        assert_eq!(convert_to_base(200.0, "EUR", "CAD", &[]), None);
    }

    #[test]
    fn same_currency_returns_amount_without_lookup() {
        assert_eq!(convert_to_base(123.45, "CAD", "CAD", &[]), Some(123.45));
        assert_eq!(convert_to_base(0.0, "USD", "USD", &[]), Some(0.0));
        // Case-insensitive match.
        assert_eq!(convert_to_base(50.0, "usd", "USD", &[]), Some(50.0));
    }

    #[test]
    fn inverted_pair_precision_within_epsilon() {
        let rates = vec![make_rate("USDCAD", 1.36)];
        let result = convert_to_base(100.0, "CAD", "USD", &rates).unwrap();
        let expected = 100.0_f64 / 1.36;
        assert!(
            (result - expected).abs() < 1e-6,
            "inverted pair result {result} differed from expected {expected} by more than 1e-6"
        );
    }
}
