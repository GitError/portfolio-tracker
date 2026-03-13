use chrono::Utc;
use reqwest::Client;

use crate::types::PriceData;

pub async fn fetch_price(client: &Client, symbol: &str) -> Result<PriceData, String> {
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=1d",
        symbol
    );

    let response = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        )
        .send()
        .await
        .map_err(|e| format!("Request failed for {}: {}", symbol, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "HTTP {} for symbol {}",
            response.status(),
            symbol
        ));
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

    let previous_close = meta["chartPreviousClose"]
        .as_f64()
        .or_else(|| meta["previousClose"].as_f64())
        .unwrap_or(price);

    let change = price - previous_close;
    let change_percent = if previous_close != 0.0 {
        (change / previous_close) * 100.0
    } else {
        0.0
    };

    let currency = meta["currency"]
        .as_str()
        .unwrap_or("USD")
        .to_string();

    Ok(PriceData {
        symbol: symbol.to_string(),
        price,
        currency,
        change,
        change_percent,
        updated_at: Utc::now().to_rfc3339(),
    })
}

pub async fn fetch_all_prices(client: &Client, symbols: Vec<String>) -> Vec<PriceData> {
    let futures: Vec<_> = symbols
        .iter()
        .map(|symbol| fetch_price(client, symbol))
        .collect();

    let results = futures::future::join_all(futures).await;

    results
        .into_iter()
        .zip(symbols.iter())
        .filter_map(|(result, symbol)| match result {
            Ok(price) => Some(price),
            Err(e) => {
                eprintln!("Failed to fetch price for {}: {}", symbol, e);
                None
            }
        })
        .collect()
}
