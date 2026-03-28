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
    fetch_price_internal(client, symbol, fallback_currency, YAHOO_CHART_URL).await
}

/// Internal implementation that accepts a configurable URL template.
/// Exposed to the test module so mockito can intercept requests.
async fn fetch_price_internal(
    client: &Client,
    symbol: &str,
    fallback_currency: Option<&str>,
    url_template: &str,
) -> Result<PriceData, String> {
    let url = url_template.replace("{}", symbol);

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
            tracing::warn!(
                "Yahoo Finance omitted currency for {}; using {:?} as fallback",
                symbol,
                used
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
    use futures::StreamExt;
    // Eagerly construct all futures into a Vec (resolving borrows of `client`,
    // `symbols`, and `symbol_currencies` before the stream runs), then drive
    // them with buffer_unordered(5) to cap concurrent HTTP connections at 5.
    let futures: Vec<_> = symbols
        .iter()
        .map(|symbol| {
            let fallback = symbol_currencies.get(symbol).map(String::as_str);
            fetch_price_with_fallback_currency(client, symbol, fallback)
        })
        .collect();
    let results: Vec<_> = futures::stream::iter(futures)
        .buffer_unordered(5)
        .collect()
        .await;

    let mut prices = Vec::new();
    let mut failed = Vec::new();

    for (result, symbol) in results.into_iter().zip(symbols.iter()) {
        match result {
            Ok(price) => prices.push(price),
            Err(e) => {
                tracing::error!("Failed to fetch price for {}: {}", symbol, e);
                failed.push(symbol.clone());
            }
        }
    }

    FetchAllPricesResult { prices, failed }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client() -> Client {
        Client::builder()
            .build()
            .expect("Failed to build reqwest client")
    }

    /// Build a Yahoo Finance chart API URL template pointing at the mock server.
    /// The `{}` placeholder will be replaced with the symbol at call time.
    fn mock_url_template(server: &mockito::Server) -> String {
        format!(
            "{}/v8/finance/chart/{{}}?interval=1d&range=1d",
            server.url()
        )
    }

    fn valid_chart_response(symbol: &str, price: f64, prev_close: f64, currency: &str) -> String {
        serde_json::json!({
            "chart": {
                "result": [{
                    "meta": {
                        "symbol": symbol,
                        "regularMarketPrice": price,
                        "chartPreviousClose": prev_close,
                        "currency": currency,
                        "regularMarketOpen": price,
                        "regularMarketVolume": 12345678_i64
                    },
                    "timestamp": [],
                    "indicators": {}
                }],
                "error": null
            }
        })
        .to_string()
    }

    // ── Test 1: Successful price fetch ────────────────────────────────────────

    #[tokio::test]
    async fn fetch_price_success_parses_price_and_change_percent() {
        let mut server = mockito::Server::new_async().await;
        let price = 195.89_f64;
        let prev_close = 193.12_f64;
        let body = valid_chart_response("AAPL", price, prev_close, "USD");

        let _mock = server
            .mock("GET", "/v8/finance/chart/AAPL?interval=1d&range=1d")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(&body)
            .create_async()
            .await;

        let client = make_client();
        let url_template = mock_url_template(&server);
        let result = fetch_price_internal(&client, "AAPL", None, &url_template).await;

        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let pd = result.unwrap();
        assert_eq!(pd.symbol, "AAPL");
        assert!((pd.price - price).abs() < 0.0001, "price mismatch");
        assert_eq!(pd.currency, "USD");
        let expected_change = price - prev_close;
        let expected_pct = (expected_change / prev_close) * 100.0;
        assert!(
            (pd.change_percent - expected_pct).abs() < 0.001,
            "change_percent mismatch: got {}, expected {}",
            pd.change_percent,
            expected_pct
        );
    }

    // ── Test 2: 403 Forbidden (missing User-Agent scenario) ───────────────────

    #[tokio::test]
    async fn fetch_price_403_returns_error() {
        let mut server = mockito::Server::new_async().await;

        let _mock = server
            .mock("GET", "/v8/finance/chart/TSLA?interval=1d&range=1d")
            .with_status(403)
            .with_body("Forbidden")
            .create_async()
            .await;

        let client = make_client();
        let url_template = mock_url_template(&server);
        let result = fetch_price_internal(&client, "TSLA", None, &url_template).await;

        assert!(result.is_err(), "expected Err on 403");
        let err = result.unwrap_err();
        assert!(
            err.contains("403") || err.contains("HTTP"),
            "error should mention HTTP 403, got: {}",
            err
        );
    }

    // ── Test 3: 404 / symbol not found ────────────────────────────────────────

    #[tokio::test]
    async fn fetch_price_404_returns_error() {
        let mut server = mockito::Server::new_async().await;

        let _mock = server
            .mock("GET", "/v8/finance/chart/INVALID?interval=1d&range=1d")
            .with_status(404)
            .with_body("Not Found")
            .create_async()
            .await;

        let client = make_client();
        let url_template = mock_url_template(&server);
        let result = fetch_price_internal(&client, "INVALID", None, &url_template).await;

        assert!(result.is_err(), "expected Err on 404");
        let err = result.unwrap_err();
        assert!(
            err.contains("404") || err.contains("HTTP"),
            "error should mention HTTP 404, got: {}",
            err
        );
    }

    // ── Test 4: Malformed JSON ────────────────────────────────────────────────

    #[tokio::test]
    async fn fetch_price_malformed_json_returns_error() {
        let mut server = mockito::Server::new_async().await;

        let _mock = server
            .mock("GET", "/v8/finance/chart/AAPL?interval=1d&range=1d")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{ this is not valid JSON }")
            .create_async()
            .await;

        let client = make_client();
        let url_template = mock_url_template(&server);
        let result = fetch_price_internal(&client, "AAPL", None, &url_template).await;

        assert!(result.is_err(), "expected Err on malformed JSON");
        let err = result.unwrap_err();
        assert!(
            err.contains("Failed to parse JSON") || err.contains("parse"),
            "error should mention JSON parse failure, got: {}",
            err
        );
    }

    // ── Test 5: Empty result array ────────────────────────────────────────────

    #[tokio::test]
    async fn fetch_price_empty_result_array_returns_error() {
        let mut server = mockito::Server::new_async().await;

        let body = serde_json::json!({
            "chart": {
                "result": [],
                "error": null
            }
        })
        .to_string();

        let _mock = server
            .mock("GET", "/v8/finance/chart/AAPL?interval=1d&range=1d")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(&body)
            .create_async()
            .await;

        let client = make_client();
        let url_template = mock_url_template(&server);
        let result = fetch_price_internal(&client, "AAPL", None, &url_template).await;

        assert!(result.is_err(), "expected Err on empty result array");
        let err = result.unwrap_err();
        assert!(
            err.contains("Missing") || err.contains("meta"),
            "error should mention missing chart meta, got: {}",
            err
        );
    }

    // ── Test 6: Missing currency field uses fallback ──────────────────────────

    #[tokio::test]
    async fn fetch_price_missing_currency_uses_fallback() {
        let mut server = mockito::Server::new_async().await;

        // Response omits the "currency" field entirely
        let body = serde_json::json!({
            "chart": {
                "result": [{
                    "meta": {
                        "symbol": "XIU.TO",
                        "regularMarketPrice": 34.5_f64,
                        "chartPreviousClose": 34.0_f64,
                        "regularMarketOpen": 34.2_f64,
                        "regularMarketVolume": 500000_i64
                        // no "currency" field
                    },
                    "timestamp": [],
                    "indicators": {}
                }],
                "error": null
            }
        })
        .to_string();

        let _mock = server
            .mock("GET", "/v8/finance/chart/XIU.TO?interval=1d&range=1d")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(&body)
            .create_async()
            .await;

        let client = make_client();
        let url_template = mock_url_template(&server);
        let result = fetch_price_internal(&client, "XIU.TO", Some("CAD"), &url_template).await;

        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let pd = result.unwrap();
        assert_eq!(
            pd.currency, "CAD",
            "fallback currency should be used when Yahoo omits it"
        );
    }
}
