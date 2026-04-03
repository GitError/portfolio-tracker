use chrono::Utc;
use reqwest::Client;

use crate::price::fetch_price;
use crate::types::FxRate;

pub async fn fetch_fx_rate(client: &Client, from: &str, base: &str) -> Result<FxRate, String> {
    let symbol = format!("{}{}=X", from, base);
    let price_data = fetch_price(client, &symbol).await?;

    Ok(FxRate {
        pair: format!("{}{}", from.to_uppercase(), base.to_uppercase()),
        rate: price_data.price,
        updated_at: Utc::now().to_rfc3339(),
    })
}

pub async fn fetch_all_fx_rates(
    client: &Client,
    currencies: Vec<String>,
    base: &str,
) -> Vec<FxRate> {
    let base_upper = base.to_uppercase();
    let non_base: Vec<String> = currencies
        .into_iter()
        .filter(|c| c.to_uppercase() != base_upper)
        .collect();

    use futures::StreamExt;
    // Eagerly construct all futures into a Vec (resolving borrows of `client`
    // and `base` before the stream runs), then drive them with
    // buffer_unordered(5) to cap concurrent HTTP connections at 5.
    let futures: Vec<_> = non_base
        .iter()
        .map(|currency| fetch_fx_rate(client, currency, base))
        .collect();
    let results: Vec<_> = futures::stream::iter(futures)
        .buffer_unordered(5)
        .collect()
        .await;

    results
        .into_iter()
        .zip(non_base.iter())
        .filter_map(|(result, currency)| match result {
            Ok(rate) => Some(rate),
            Err(e) => {
                tracing::error!("Failed to fetch FX rate for {}{}: {}", currency, base, e);
                None
            }
        })
        .collect()
}

/// Convert `amount` from `from_currency` into `base` using the cached `rates`.
///
/// Returns `Some(converted)` when a rate is available, or `None` when no
/// matching rate exists in the cache.  Callers should treat `None` as a signal
/// that the conversion is unreliable and surface a stale-FX warning to the
/// user rather than silently falling back to a 1:1 rate.
pub fn convert_to_base(
    amount: f64,
    from_currency: &str,
    base: &str,
    rates: &[FxRate],
) -> Option<f64> {
    let from_upper = from_currency.to_uppercase();
    let base_upper = base.to_uppercase();
    if from_upper == base_upper {
        return Some(amount);
    }

    // Try the direct pair first: e.g. USDCAD when converting USD → CAD
    let direct_pair = format!("{}{}", from_upper, base_upper);
    if let Some(rate) = rates.iter().find(|r| r.pair == direct_pair) {
        if rate.rate == 0.0 {
            tracing::warn!(pair = %rate.pair, "FX rate is zero; returning amount with fxStale=true");
            return None;
        }
        return Some(amount * rate.rate);
    }

    // Fall back to the inverted pair: e.g. CADUSD when converting USD → CAD
    // but base=CAD was previously cached as USDCAD.  Invert the stored rate.
    let inverted_pair = format!("{}{}", base_upper, from_upper);
    if let Some(rate) = rates.iter().find(|r| r.pair == inverted_pair) {
        if rate.rate != 0.0 {
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
            "zero direct-pair rate should return None to trigger fx_stale, not Some(0)"
        );
    }

    #[test]
    fn inverted_pair_zero_rate_returns_none() {
        // An inverted pair with rate == 0.0 must also return None (existing guard).
        let rates = vec![make_rate("CADUSD", 0.0)];
        let result = convert_to_base(100.0, "CAD", "USD", &rates);
        assert_eq!(
            result, None,
            "zero inverted-pair rate should return None to trigger fx_stale, not a divide-by-zero"
        );
    }

    #[test]
    fn zero_rate_direct_pair_returns_none_stale() {
        // Direct pair with rate == 0.0 must return None (fx_stale) — not Some(0.0).
        let rates = vec![make_rate("USDCAD", 0.0)];
        assert_eq!(
            convert_to_base(250.0, "USD", "CAD", &rates),
            None,
            "zero direct-pair rate should signal fx_stale via None"
        );
    }

    #[test]
    fn zero_rate_inverted_pair_returns_none_stale() {
        // Inverted pair with rate == 0.0 must also return None to avoid divide-by-zero.
        let rates = vec![make_rate("CADUSD", 0.0)];
        assert_eq!(
            convert_to_base(250.0, "CAD", "USD", &rates),
            None,
            "zero inverted-pair rate should signal fx_stale via None"
        );
    }

    #[test]
    fn unknown_currency_pair_returns_none_stale() {
        // Neither GBP/CAD nor CAD/GBP in cache → None (pass-through with stale flag).
        let rates = vec![make_rate("USDCAD", 1.36)];
        assert_eq!(
            convert_to_base(100.0, "GBP", "CAD", &rates),
            None,
            "unknown pair should return None to trigger fx_stale"
        );
        // Empty cache also returns None.
        assert_eq!(convert_to_base(100.0, "EUR", "CAD", &[]), None);
    }

    #[test]
    fn same_currency_returns_amount_without_lookup() {
        // CAD → CAD should short-circuit and return the amount unchanged (no rates needed).
        assert_eq!(convert_to_base(123.45, "CAD", "CAD", &[]), Some(123.45));
        assert_eq!(convert_to_base(0.0, "USD", "USD", &[]), Some(0.0));
        // Case-insensitive match.
        assert_eq!(convert_to_base(50.0, "usd", "USD", &[]), Some(50.0));
    }

    #[test]
    fn inverted_pair_precision_within_epsilon() {
        // Inverted rate: 1 / 1.36 ≈ 0.735294…  Result should match within 1e-6.
        let rates = vec![make_rate("USDCAD", 1.36)];
        let result = convert_to_base(100.0, "CAD", "USD", &rates).unwrap();
        let expected = 100.0_f64 / 1.36;
        assert!(
            (result - expected).abs() < 1e-6,
            "inverted pair result {result} differed from expected {expected} by more than 1e-6"
        );
    }

    #[test]
    fn fetch_all_fx_rates_filters_base_currency() {
        let currencies = vec!["USD".to_string(), "CAD".to_string(), "EUR".to_string()];
        let base = "CAD";
        let base_upper = base.to_uppercase();
        let non_base: Vec<String> = currencies
            .into_iter()
            .filter(|c| c.to_uppercase() != base_upper)
            .collect();
        assert_eq!(non_base, vec!["USD", "EUR"]);
    }

    #[test]
    fn fetch_all_fx_rates_filters_usd_base() {
        let currencies = vec!["CAD".to_string(), "USD".to_string(), "EUR".to_string()];
        let base = "USD";
        let base_upper = base.to_uppercase();
        let non_base: Vec<String> = currencies
            .into_iter()
            .filter(|c| c.to_uppercase() != base_upper)
            .collect();
        assert_eq!(non_base, vec!["CAD", "EUR"]);
    }
}
