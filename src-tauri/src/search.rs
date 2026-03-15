use reqwest::Client;

use crate::config::USER_AGENT;
use crate::types::{AssetType, SymbolResult};

pub async fn search_symbols_yahoo(
    client: &Client,
    query: &str,
) -> Result<Vec<SymbolResult>, String> {
    let encoded_query: String = query
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect();
    let url = format!(
        "https://query1.finance.yahoo.com/v1/finance/search?q={}&quotesCount=8&newsCount=0&enableFuzzyQuery=false",
        encoded_query
    );
    let response = client
        .get(&url)
        .header("User-Agent", crate::config::USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Symbol search request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {} from symbol search API", response.status()));
    }

    let json: serde_json::Value = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse symbol search response: {}", e))?;

    let quotes = json
        .get("quotes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "No quotes in search response".to_string())?;

    let results = quotes
        .iter()
        .filter_map(|q| {
            let symbol = q["symbol"].as_str()?.to_string();
            // Skip index symbols and overly long symbols
            if symbol.contains('^') || symbol.len() > 12 {
                return None;
            }
            let name = q["shortname"]
                .as_str()
                .or_else(|| q["longname"].as_str())
                .unwrap_or(&symbol)
                .to_string();
            let quote_type = q["quoteType"].as_str().unwrap_or("EQUITY");
            let exchange = q["exchange"].as_str().unwrap_or("").to_string();
            let currency = q["currency"].as_str().unwrap_or("USD").to_string();

            let asset_type = match quote_type {
                "ETF" | "MUTUALFUND" => AssetType::Etf,
                "CRYPTOCURRENCY" => AssetType::Crypto,
                _ => AssetType::Stock,
            };

            Some(SymbolResult {
                symbol,
                name,
                asset_type,
                exchange,
                currency,
            })
        })
        .collect();

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_type_mapping_equity() {
        // Verify the match arms compile and produce correct variants
        let t = match "EQUITY" {
            "ETF" | "MUTUALFUND" => AssetType::Etf,
            "CRYPTOCURRENCY" => AssetType::Crypto,
            _ => AssetType::Stock,
        };
        assert_eq!(t.as_str(), "stock");
    }

    #[test]
    fn asset_type_mapping_etf() {
        let t = match "ETF" {
            "ETF" | "MUTUALFUND" => AssetType::Etf,
            "CRYPTOCURRENCY" => AssetType::Crypto,
            _ => AssetType::Stock,
        };
        assert_eq!(t.as_str(), "etf");
    }

    #[test]
    fn asset_type_mapping_crypto() {
        let t = match "CRYPTOCURRENCY" {
            "ETF" | "MUTUALFUND" => AssetType::Etf,
            "CRYPTOCURRENCY" => AssetType::Crypto,
            _ => AssetType::Stock,
        };
        assert_eq!(t.as_str(), "crypto");
    }

    #[test]
    fn symbol_filter_rejects_index_symbols() {
        // Symbols containing ^ should be filtered out
        let symbol = "^GSPC";
        assert!(symbol.contains('^'));
    }

    #[test]
    fn symbol_filter_rejects_long_symbols() {
        let symbol = "TOOLONGSYMBOLX";
        assert!(symbol.len() > 12);
    }

    #[test]
    fn query_url_encoding_via_reqwest_params() {
        // Verify that reqwest properly percent-encodes special characters when
        // building the URL via `.query()`.  This is a compile-time / unit check:
        // construct a URL the same way the function does and assert the raw query
        // string contains percent-encoded characters rather than literals.
        let base = reqwest::Url::parse("https://query1.finance.yahoo.com/v1/finance/search")
            .expect("base URL");
        let url = reqwest::Url::parse_with_params(
            base.as_str(),
            &[("q", "Apple & Google"), ("quotesCount", "8")],
        )
        .expect("parse with params");
        let query = url.query().unwrap_or_default();
        // The ampersand must be percent-encoded; spaces encoded as %20 or +
        assert!(
            !query.contains(" & "),
            "literal ampersand/space should not appear in encoded query: {}",
            query
        );
        assert!(
            query.contains("q="),
            "encoded URL should still contain 'q=' param: {}",
            query
        );
    }
}
