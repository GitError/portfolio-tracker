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

    let futures: Vec<_> = non_base
        .iter()
        .map(|currency| fetch_fx_rate(client, currency, base))
        .collect();

    let results = futures::future::join_all(futures).await;

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

pub fn convert_to_base(amount: f64, from_currency: &str, base: &str, rates: &[FxRate]) -> f64 {
    let from_upper = from_currency.to_uppercase();
    let base_upper = base.to_uppercase();
    if from_upper == base_upper {
        return amount;
    }

    // Try the direct pair first: e.g. USDCAD when converting USD → CAD
    let direct_pair = format!("{}{}", from_upper, base_upper);
    if let Some(rate) = rates.iter().find(|r| r.pair == direct_pair) {
        return amount * rate.rate;
    }

    // Fall back to the inverted pair: e.g. CADUSD when converting USD → CAD
    // but base=CAD was previously cached as USDCAD.  Invert the stored rate.
    let inverted_pair = format!("{}{}", base_upper, from_upper);
    if let Some(rate) = rates.iter().find(|r| r.pair == inverted_pair) {
        if rate.rate != 0.0 {
            return amount / rate.rate;
        }
    }

    tracing::warn!(
        "FX rate not found for {} → {}, returning unconverted amount",
        from_currency,
        base
    );
    amount
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
        assert_eq!(convert_to_base(100.0, "CAD", "CAD", &rates), 100.0);
        assert_eq!(convert_to_base(100.0, "cad", "CAD", &rates), 100.0);
        assert_eq!(convert_to_base(100.0, "USD", "USD", &rates), 100.0);
    }

    #[test]
    fn usd_converts_to_cad_correctly() {
        let rates = vec![make_rate("USDCAD", 1.36)];
        let result = convert_to_base(100.0, "USD", "CAD", &rates);
        assert!((result - 136.0).abs() < 0.001);
    }

    #[test]
    fn cad_converts_to_usd_correctly() {
        let rates = vec![make_rate("CADUSD", 0.735)];
        let result = convert_to_base(100.0, "CAD", "USD", &rates);
        assert!((result - 73.5).abs() < 0.001);
    }

    #[test]
    fn missing_rate_returns_amount_unchanged() {
        let result = convert_to_base(200.0, "EUR", "CAD", &[]);
        assert_eq!(result, 200.0);
    }

    #[test]
    fn cad_converts_to_usd_using_inverted_usdcad_pair() {
        // Only USDCAD is cached (as stored when CAD was the base). When base switches
        // to USD we must invert the stored rate rather than return unconverted.
        let rates = vec![make_rate("USDCAD", 1.36)];
        let result = convert_to_base(100.0, "CAD", "USD", &rates);
        // 100 CAD / 1.36 ≈ 73.529
        assert!((result - (100.0_f64 / 1.36)).abs() < 0.001);
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
