use chrono::Utc;
use reqwest::Client;

use crate::config::{USER_AGENT, YAHOO_CHART_URL};
use crate::types::PriceData;

pub async fn fetch_price(client: &Client, symbol: &str) -> Result<PriceData, String> {
    fetch_price_with_fallback_currency(client, symbol, None).await
}

/// Like [`fetch_price`] but accepts an optional `fallback_currency` that is
/// used when Yahoo Finance omits the `currency` field in its response.
/// Providing the holding's own stored currency avoids silently mislabelling
/// CAD-listed (or other non-USD) symbols as USD.
pub async fn fetch_price_with_fallback_currency(
    client: &Client,
    symbol: &str,
    fallback_currency: Option<&str>,
) -> Result<PriceData, String> {
    let url = YAHOO_CHART_URL.replace("{}", symbol);

    let response = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Request failed for {}: {}", symbol, e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {} for symbol {}", response.status(), symbol));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse JSON for {}: {}", symbol, e))?;

    let meta = json
        .pointer("/chart/result/0/meta")
        .ok_or_else(|| format!("Missing chart.result[0].meta for {}", symbol))?;

    let price = meta["regularMarketPrice"]
        .as_f64()
        .ok_or_else(|| format!("Missing regularMarketPrice for {}", symbol))?;

    let previous_close_val = meta["chartPreviousClose"]
        .as_f64()
        .or_else(|| meta["previousClose"].as_f64());

    let open = meta["regularMarketOpen"].as_f64();
    let volume = meta["regularMarketVolume"].as_i64();

    let change_base = previous_close_val.unwrap_or(price);
    let change = price - change_base;
    let change_percent = if change_base != 0.0 {
        (change / change_base) * 100.0
    } else {
        0.0
    };

    let currency = match meta["currency"].as_str() {
        Some(c) => c.to_string(),
        None => {
            let used = fallback_currency.unwrap_or("USD");
            eprintln!(
                "Warning: Yahoo Finance omitted currency for {}; using {:?} as fallback",
                symbol, used
            );
            used.to_string()
        }
    };

    Ok(PriceData {
        symbol: symbol.to_string(),
        price,
        currency,
        change,
        change_percent,
        updated_at: Utc::now().to_rfc3339(),
        open,
        previous_close: previous_close_val,
        volume,
    })
}

/// Result of a bulk price fetch.
pub struct FetchAllPricesResult {
    pub prices: Vec<PriceData>,
    /// Symbols for which the fetch failed (network error, bad HTTP status, parse failure).
    pub failed: Vec<String>,
}

/// Fetch prices for all symbols in parallel.
/// `symbol_currencies` maps each symbol to its holding currency so that when
/// Yahoo Finance omits the `currency` field the holding's own currency is used
/// as fallback instead of silently assuming USD.
pub async fn fetch_all_prices(
    client: &Client,
    symbols: Vec<String>,
    symbol_currencies: &std::collections::HashMap<String, String>,
) -> FetchAllPricesResult {
    let futures: Vec<_> = symbols
        .iter()
        .map(|symbol| {
            let fallback = symbol_currencies.get(symbol).map(String::as_str);
            fetch_price_with_fallback_currency(client, symbol, fallback)
        })
        .collect();

    let results = futures::future::join_all(futures).await;

    let mut prices = Vec::new();
    let mut failed = Vec::new();

    for (result, symbol) in results.into_iter().zip(symbols.iter()) {
        match result {
            Ok(price) => prices.push(price),
            Err(e) => {
                eprintln!("Failed to fetch price for {}: {}", symbol, e);
                failed.push(symbol.clone());
            }
        }
    }

    FetchAllPricesResult { prices, failed }
}
