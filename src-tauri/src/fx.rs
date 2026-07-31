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

// `convert_to_base` moved to `portfolio-core` (shared with `portfolio-mcp` — see
// #615); nothing in this crate calls it directly anymore, so it's no longer
// re-exported here. Its tests live in `portfolio-core/src/fx.rs`.

#[cfg(test)]
mod tests {
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
