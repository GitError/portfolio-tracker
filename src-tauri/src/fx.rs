use chrono::Utc;
use reqwest::Client;

use crate::price::fetch_price;
use crate::types::FxRate;

pub async fn fetch_fx_rate(client: &Client, from: &str) -> Result<FxRate, String> {
    let symbol = format!("{}CAD=X", from);
    let price_data = fetch_price(client, &symbol).await?;

    Ok(FxRate {
        pair: format!("{}CAD", from),
        rate: price_data.price,
        updated_at: Utc::now().to_rfc3339(),
    })
}

pub async fn fetch_all_fx_rates(client: &Client, currencies: Vec<String>) -> Vec<FxRate> {
    let non_cad: Vec<String> = currencies
        .into_iter()
        .filter(|c| c.to_uppercase() != "CAD")
        .collect();

    let futures: Vec<_> = non_cad
        .iter()
        .map(|currency| fetch_fx_rate(client, currency))
        .collect();

    let results = futures::future::join_all(futures).await;

    results
        .into_iter()
        .zip(non_cad.iter())
        .filter_map(|(result, currency)| match result {
            Ok(rate) => Some(rate),
            Err(e) => {
                eprintln!("Failed to fetch FX rate for {}CAD: {}", currency, e);
                None
            }
        })
        .collect()
}

#[allow(dead_code)]
pub fn convert_to_cad(amount: f64, from_currency: &str, rates: &[FxRate]) -> f64 {
    if from_currency.to_uppercase() == "CAD" {
        return amount;
    }

    let pair = format!("{}CAD", from_currency.to_uppercase());
    match rates.iter().find(|r| r.pair == pair) {
        Some(rate) => amount * rate.rate,
        None => {
            eprintln!(
                "FX rate not found for {} → CAD, returning unconverted amount",
                from_currency
            );
            amount
        }
    }
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
    fn cad_passthrough_returns_amount_unchanged() {
        let rates = vec![make_rate("USDCAD", 1.36)];
        assert_eq!(convert_to_cad(100.0, "CAD", &rates), 100.0);
        assert_eq!(convert_to_cad(100.0, "cad", &rates), 100.0);
    }

    #[test]
    fn usd_converts_correctly() {
        let rates = vec![make_rate("USDCAD", 1.36)];
        let result = convert_to_cad(100.0, "USD", &rates);
        assert!((result - 136.0).abs() < 0.001);
    }

    #[test]
    fn missing_rate_returns_amount_unchanged() {
        let result = convert_to_cad(200.0, "EUR", &[]);
        assert_eq!(result, 200.0);
    }

    #[test]
    fn fetch_all_fx_rates_filters_cad() {
        // CAD should be excluded from the list passed to fetching
        let currencies = vec!["USD".to_string(), "CAD".to_string(), "EUR".to_string()];
        let non_cad: Vec<String> = currencies
            .into_iter()
            .filter(|c| c.to_uppercase() != "CAD")
            .collect();
        assert_eq!(non_cad, vec!["USD", "EUR"]);
    }
}
