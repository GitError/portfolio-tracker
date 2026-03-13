use chrono::Utc;
use reqwest::Client;

use crate::types::FxRate;
use crate::price::fetch_price;

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
