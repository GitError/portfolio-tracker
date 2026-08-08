use chrono::Utc;
use reqwest::Client;

use crate::config::{USER_AGENT, YAHOO_CHART_URL, YAHOO_QUOTE_URL};
use crate::types::PriceData;

/// Validate a ticker symbol before it is interpolated into a Yahoo Finance URL.
///
/// Restricts symbols to the character set Yahoo Finance actually uses (uppercase
/// letters, digits, `.` for exchange suffixes like `.TO`, `-` for pairs like
/// `BTC-USD`, `^` for indices like `^GSPC`, and `=` for futures/FX like `CL=F`
/// or `EURUSD=X`), rejecting anything else — including path-traversal sequences
/// like `../../etc/passwd` — before it ever reaches a URL.
pub(crate) fn validate_symbol(symbol: &str) -> Result<(), String> {
    let valid = !symbol.is_empty()
        && symbol.len() <= 20
        && symbol.chars().all(|c| {
            c.is_ascii_uppercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '^' | '=')
        });
    if valid {
        Ok(())
    } else {
        Err(format!("Invalid symbol format: {:?}", symbol))
    }
}

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
    validate_symbol(symbol)?;
    let encoded_symbol = urlencoding::encode(symbol);
    let url = url_template.replace("{}", &encoded_symbol);

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

/// Research-watchlist market data snapshot, parsed from Yahoo Finance's v7
/// bulk quote endpoint (the same endpoint `analytics::get_symbol_metadata_with_cache`
/// uses for market cap / P/E / dividend yield). Every field is optional because
/// Yahoo omits fields per quote type (e.g. ETFs and cash-like symbols lack a P/E).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WatchlistSnapshotData {
    pub name: Option<String>,
    pub price: Option<f64>,
    pub currency: Option<String>,
    pub market_cap: Option<f64>,
    pub fifty_two_week_low: Option<f64>,
    pub fifty_two_week_high: Option<f64>,
    pub ytd_return: Option<f64>,
    pub one_year_return: Option<f64>,
    pub dividend_yield: Option<f64>,
    pub pe_ratio: Option<f64>,
}

/// Fetch a single symbol's research-watchlist market data snapshot.
pub async fn fetch_watchlist_snapshot(
    client: &Client,
    symbol: &str,
) -> Result<WatchlistSnapshotData, String> {
    fetch_watchlist_snapshot_internal(client, symbol, YAHOO_QUOTE_URL).await
}

/// Internal implementation accepting a configurable URL template so tests can
/// point it at a mock server, mirroring `fetch_price_internal`.
async fn fetch_watchlist_snapshot_internal(
    client: &Client,
    symbol: &str,
    url_template: &str,
) -> Result<WatchlistSnapshotData, String> {
    validate_symbol(symbol)?;
    let encoded_symbol = urlencoding::encode(symbol);
    let url = url_template.replace("{}", &encoded_symbol);

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

    let item = json
        .pointer("/quoteResponse/result/0")
        .ok_or_else(|| format!("No quote data returned for {}", symbol))?;

    Ok(WatchlistSnapshotData {
        name: item
            .get("longName")
            .or_else(|| item.get("shortName"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        price: item.get("regularMarketPrice").and_then(|v| v.as_f64()),
        currency: item
            .get("currency")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        market_cap: item.get("marketCap").and_then(|v| v.as_f64()),
        fifty_two_week_low: item.get("fiftyTwoWeekLow").and_then(|v| v.as_f64()),
        fifty_two_week_high: item.get("fiftyTwoWeekHigh").and_then(|v| v.as_f64()),
        ytd_return: item.get("ytdReturn").and_then(|v| v.as_f64()),
        one_year_return: item
            .get("fiftyTwoWeekChangePercent")
            .and_then(|v| v.as_f64()),
        dividend_yield: item
            .get("trailingAnnualDividendYield")
            .and_then(|v| v.as_f64()),
        pe_ratio: item.get("trailingPE").and_then(|v| v.as_f64()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Symbol validation ─────────────────────────────────────────────────────

    #[test]
    fn validate_symbol_accepts_normal_tickers() {
        for sym in &["AAPL", "BTC-USD", "XIU.TO", "^GSPC", "CL=F", "EURUSD=X"] {
            assert!(validate_symbol(sym).is_ok(), "expected {} to be valid", sym);
        }
    }

    #[test]
    fn validate_symbol_rejects_path_traversal() {
        assert!(validate_symbol("../../etc/passwd").is_err());
    }

    #[test]
    fn validate_symbol_rejects_url_special_characters() {
        for sym in &[
            "AAPL?x=1",
            "AAPL&y=2",
            "AAPL/../x",
            "AAPL#frag",
            "AAPL TSLA",
        ] {
            assert!(
                validate_symbol(sym).is_err(),
                "expected {} to be rejected",
                sym
            );
        }
    }

    #[test]
    fn validate_symbol_rejects_empty_and_overlong() {
        assert!(validate_symbol("").is_err());
        assert!(validate_symbol(&"A".repeat(21)).is_err());
        assert!(validate_symbol(&"A".repeat(20)).is_ok());
    }

    #[test]
    fn validate_symbol_rejects_lowercase() {
        // Yahoo symbols are always uppercase; lowercase is rejected rather than
        // silently normalized to avoid masking malformed input.
        assert!(validate_symbol("aapl").is_err());
    }

    #[tokio::test]
    async fn fetch_price_rejects_invalid_symbol_before_any_request() {
        let client = make_client();
        let result = fetch_price_internal(
            &client,
            "../../etc/passwd",
            None,
            "https://example.invalid/{}",
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid symbol"));
    }

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

    // ── fetch_watchlist_snapshot ────────────────────────────────────────────

    fn mock_quote_url_template(server: &mockito::Server) -> String {
        format!("{}/v7/finance/quote?symbols={{}}", server.url())
    }

    #[tokio::test]
    async fn fetch_watchlist_snapshot_parses_all_fields() {
        let mut server = mockito::Server::new_async().await;
        let body = serde_json::json!({
            "quoteResponse": {
                "result": [{
                    "symbol": "AAPL",
                    "longName": "Apple Inc.",
                    "regularMarketPrice": 195.89_f64,
                    "currency": "USD",
                    "marketCap": 3_000_000_000_000_i64,
                    "fiftyTwoWeekLow": 150.0_f64,
                    "fiftyTwoWeekHigh": 200.0_f64,
                    "ytdReturn": 12.5_f64,
                    "fiftyTwoWeekChangePercent": 18.3_f64,
                    "trailingAnnualDividendYield": 0.005_f64,
                    "trailingPE": 32.1_f64
                }],
                "error": null
            }
        })
        .to_string();

        let _mock = server
            .mock("GET", "/v7/finance/quote?symbols=AAPL")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(&body)
            .create_async()
            .await;

        let client = make_client();
        let url_template = mock_quote_url_template(&server);
        let result = fetch_watchlist_snapshot_internal(&client, "AAPL", &url_template).await;

        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let snap = result.unwrap();
        assert_eq!(snap.name, Some("Apple Inc.".to_string()));
        assert_eq!(snap.price, Some(195.89));
        assert_eq!(snap.currency, Some("USD".to_string()));
        assert_eq!(snap.market_cap, Some(3_000_000_000_000.0));
        assert_eq!(snap.fifty_two_week_low, Some(150.0));
        assert_eq!(snap.fifty_two_week_high, Some(200.0));
        assert_eq!(snap.ytd_return, Some(12.5));
        assert_eq!(snap.one_year_return, Some(18.3));
        assert_eq!(snap.dividend_yield, Some(0.005));
        assert_eq!(snap.pe_ratio, Some(32.1));
    }

    #[tokio::test]
    async fn fetch_watchlist_snapshot_missing_fields_are_none() {
        let mut server = mockito::Server::new_async().await;
        // An ETF-like quote that omits P/E and dividend yield.
        let body = serde_json::json!({
            "quoteResponse": {
                "result": [{
                    "symbol": "VOO",
                    "regularMarketPrice": 490.1_f64,
                    "currency": "USD"
                }],
                "error": null
            }
        })
        .to_string();

        let _mock = server
            .mock("GET", "/v7/finance/quote?symbols=VOO")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(&body)
            .create_async()
            .await;

        let client = make_client();
        let url_template = mock_quote_url_template(&server);
        let result = fetch_watchlist_snapshot_internal(&client, "VOO", &url_template).await;

        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let snap = result.unwrap();
        assert_eq!(snap.price, Some(490.1));
        assert_eq!(snap.pe_ratio, None);
        assert_eq!(snap.dividend_yield, None);
        assert_eq!(snap.market_cap, None);
    }

    #[tokio::test]
    async fn fetch_watchlist_snapshot_empty_result_array_returns_error() {
        let mut server = mockito::Server::new_async().await;
        let body = serde_json::json!({
            "quoteResponse": { "result": [], "error": null }
        })
        .to_string();

        let _mock = server
            .mock("GET", "/v7/finance/quote?symbols=INVALID")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(&body)
            .create_async()
            .await;

        let client = make_client();
        let url_template = mock_quote_url_template(&server);
        let result = fetch_watchlist_snapshot_internal(&client, "INVALID", &url_template).await;

        assert!(result.is_err(), "expected Err on empty result array");
    }

    #[tokio::test]
    async fn fetch_watchlist_snapshot_rejects_invalid_symbol_before_any_request() {
        let client = make_client();
        let result = fetch_watchlist_snapshot_internal(
            &client,
            "../../etc/passwd",
            "https://example.invalid/{}",
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid symbol"));
    }

    #[tokio::test]
    async fn fetch_watchlist_snapshot_http_error_returns_error() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v7/finance/quote?symbols=TSLA")
            .with_status(403)
            .with_body("Forbidden")
            .create_async()
            .await;

        let client = make_client();
        let url_template = mock_quote_url_template(&server);
        let result = fetch_watchlist_snapshot_internal(&client, "TSLA", &url_template).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("403"));
    }
}
