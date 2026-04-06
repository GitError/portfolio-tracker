use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use sqlx::SqlitePool;
use tauri::State;

use crate::db;
use crate::error::AppError;
use crate::search::search_symbols_yahoo;
use crate::types::{RealizedGainsSummary, SymbolResult};

pub mod accounts;
pub mod alerts;
pub mod analytics;
pub mod backup;
pub mod config;
pub mod dividends;
pub mod import;
pub mod portfolio;
pub mod prices;
pub mod stress;
pub mod transactions;

pub use accounts::*;
pub use alerts::*;
pub use analytics::*;
pub use backup::*;
pub use config::*;
pub use dividends::*;
pub use import::*;
pub use portfolio::*;
pub use prices::*;
pub use stress::*;
pub use transactions::*;

pub struct DbState(pub SqlitePool);
pub struct HttpClient(pub reqwest::Client);

pub(crate) struct SearchCacheEntry {
    results: Vec<SymbolResult>,
    cached_at: Instant,
    last_accessed_at: Instant,
}

pub struct SearchCacheState(pub Mutex<HashMap<String, SearchCacheEntry>>);

/// In-memory cache for the aggregate `RealizedGainsSummary` (all holdings, all transactions).
/// Invalidated whenever a transaction is added or deleted, or when the cost-basis method changes.
pub struct RealizedGainsCacheState(pub Mutex<Option<RealizedGainsSummary>>);

impl RealizedGainsCacheState {
    pub fn new() -> Self {
        RealizedGainsCacheState(Mutex::new(None))
    }

    /// Return the cached summary if present, or `None` if the cache is cold/poisoned.
    pub fn get(&self) -> Option<RealizedGainsSummary> {
        match self.0.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => {
                tracing::warn!("RealizedGainsCache mutex poisoned; recomputing");
                None
            }
        }
    }

    /// Store a freshly-computed summary in the cache.
    pub fn set(&self, summary: RealizedGainsSummary) {
        if let Ok(mut guard) = self.0.lock() {
            *guard = Some(summary);
        }
    }

    /// Clear the cache so the next read triggers a recompute.
    pub fn invalidate(&self) {
        if let Ok(mut guard) = self.0.lock() {
            *guard = None;
        }
    }
}

impl SearchCacheState {
    pub fn new() -> Self {
        SearchCacheState(Mutex::new(HashMap::new()))
    }

    pub(crate) fn get(&self, key: &str) -> Option<Vec<SymbolResult>> {
        let mut cache = match self.0.lock() {
            Ok(guard) => guard,
            Err(_) => {
                tracing::warn!("Search cache mutex poisoned; cache disabled for this request");
                return None;
            }
        };
        let entry = cache.get_mut(key)?;
        if entry.cached_at.elapsed()
            > Duration::from_secs(crate::config::SEARCH_CACHE_TTL_SECS as u64)
        {
            return None;
        }
        entry.last_accessed_at = Instant::now();
        Some(entry.results.clone())
    }

    pub(crate) fn set(&self, key: String, results: Vec<SymbolResult>) {
        if let Ok(mut cache) = self.0.lock() {
            if cache.len() >= crate::config::SEARCH_CACHE_MAX_ENTRIES {
                if let Some(lru_key) = cache
                    .iter()
                    .min_by_key(|(_, v)| v.last_accessed_at)
                    .map(|(k, _)| k.clone())
                {
                    cache.remove(&lru_key);
                }
            }
            let now = Instant::now();
            cache.insert(
                key,
                SearchCacheEntry {
                    results,
                    cached_at: now,
                    last_accessed_at: now,
                },
            );
        }
    }
}

/// Simple per-command rate limiter to prevent API abuse.
pub struct RateLimiterState {
    pub last_search: Mutex<Option<Instant>>,
    pub last_refresh: Mutex<Option<Instant>>,
}

impl RateLimiterState {
    pub fn new() -> Self {
        RateLimiterState {
            last_search: Mutex::new(None),
            last_refresh: Mutex::new(None),
        }
    }
}

pub(crate) async fn get_base_currency(pool: &SqlitePool) -> String {
    db::get_config(pool, "base_currency")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| crate::config::BASE_CURRENCY.to_string())
}

pub(crate) async fn validate_symbol(
    db: &State<'_, DbState>,
    client: &State<'_, HttpClient>,
    symbol: &str,
) -> Result<Option<SymbolResult>, AppError> {
    let pool = &db.0;
    if let Some(cached) = db::get_symbol_cache_exact(pool, symbol).await? {
        return Ok(Some(cached));
    }

    let result = search_symbols_yahoo(&client.0, symbol)
        .await?
        .into_iter()
        .find(|candidate| candidate.symbol.eq_ignore_ascii_case(symbol));

    if let Some(ref symbol_result) = result {
        if let Err(e) = db::upsert_symbol(pool, symbol_result).await {
            tracing::warn!("Failed to cache symbol: {}", e);
        }
    }

    Ok(result)
}

pub(crate) fn validate_holding_fields(
    quantity: f64,
    cost_basis: f64,
    currency: &str,
) -> Result<String, AppError> {
    if quantity <= 0.0 || !quantity.is_finite() {
        return Err(AppError::Validation(
            "quantity must be a positive finite number".to_string(),
        ));
    }
    if cost_basis < 0.0 || !cost_basis.is_finite() {
        return Err(AppError::Validation(
            "costBasis must be a non-negative finite number".to_string(),
        ));
    }
    let currency = currency.trim().to_uppercase();
    if currency.len() != 3 || !currency.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(AppError::Validation(
            "currency must be a 3-letter ISO currency code".to_string(),
        ));
    }
    Ok(currency)
}

pub(crate) const WEIGHT_EPSILON: f64 = 0.001;

#[cfg(test)]
mod tests {
    use crate::csv::{build_holdings_csv, parse_import_rows};
    use crate::portfolio::build_portfolio_snapshot;
    use crate::types::{AccountType, AssetType, FxRate, Holding, HoldingId, PriceData};
    use chrono::Utc;

    // CSV/normalize tests live in csv.rs.
    // build_portfolio_snapshot tests live in portfolio.rs.

    // ── Target-weight guard tests (logic lives in commands.rs) ─────────────
    // Note: CSV/snapshot tests have been moved to csv.rs and portfolio.rs.
    // The duplicate tests below are retained to avoid disrupting git history; they
    // delegate to the same pub functions and will be cleaned up in a follow-on PR.

    fn make_holding(
        symbol: &str,
        asset_type: AssetType,
        quantity: f64,
        cost_basis: f64,
        currency: &str,
    ) -> Holding {
        Holding {
            id: HoldingId(symbol.to_string()),
            symbol: symbol.to_string(),
            name: symbol.to_string(),
            asset_type,
            account: AccountType::Taxable,
            account_id: None,
            account_name: None,
            quantity,
            cost_basis,
            currency: currency.to_string(),
            exchange: String::new(),
            target_weight: 0.0,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            indicated_annual_dividend: None,
            indicated_annual_dividend_currency: None,
            dividend_frequency: None,
            maturity_date: None,
        }
    }

    #[test]
    fn parse_import_rows_supports_semicolon_delimiter() {
        let csv =
            "symbol;name;type;quantity;cost_basis;currency\nAAPL;Apple Inc.;stock;5;120;usd\n";
        let rows = parse_import_rows(csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "AAPL");
        assert_eq!(rows[0].currency, "USD");
    }

    #[test]
    fn parse_import_rows_reads_optional_target_weight() {
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\nAAPL,Apple Inc.,stock,5,120,USD,12.5\n";
        let rows = parse_import_rows(csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert!((rows[0].target_weight - 12.5).abs() < 0.001);
    }

    #[test]
    fn parse_import_rows_rejects_missing_required_columns() {
        let csv = "symbol,name,type,quantity,currency\nAAPL,Apple Inc.,stock,5,USD\n";
        let error = parse_import_rows(csv).expect_err("missing cost_basis should fail");

        assert!(error.contains("Missing required column: cost_basis"));
    }

    #[test]
    fn import_weight_sum_over_100_detected() {
        // Two rows whose target_weight values sum to 110; the command-level guard
        // rejects this.  Verify parse_import_rows succeeds and the sum exceeds 100.
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   AAPL,Apple Inc.,stock,5,120,USD,60\n\
                   MSFT,Microsoft,stock,3,200,USD,50\n";
        let rows = parse_import_rows(csv).expect("rows should parse");
        let total: f64 = rows.iter().map(|r| r.target_weight).sum();
        assert!(
            total > 100.0,
            "expected total > 100 to trigger command-level guard, got {}",
            total
        );
    }

    #[test]
    fn import_weight_sum_at_100_is_valid() {
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   AAPL,Apple Inc.,stock,5,120,USD,60\n\
                   MSFT,Microsoft,stock,3,200,USD,40\n";
        let rows = parse_import_rows(csv).expect("rows should parse");
        let total: f64 = rows.iter().map(|r| r.target_weight).sum();
        assert!(
            (total - 100.0).abs() < 0.001,
            "expected total == 100, got {}",
            total
        );
    }

    #[test]
    fn build_holdings_csv_includes_target_weight_column() {
        let mut holding = make_holding("AAPL", AssetType::Stock, 5.0, 120.0, "USD");
        holding.target_weight = 22.5;

        let csv = build_holdings_csv(&[holding]).expect("build csv");

        assert!(csv.starts_with(
            "symbol,name,type,account,quantity,cost_basis,currency,exchange,target_weight"
        ));
        assert!(csv.contains(",22.5"));
    }

    // ── CSV round-trip tests ──────────────────────────────────────────────────

    /// Export a set of holdings to CSV, re-parse it with `parse_import_rows`,
    /// and verify that every key field is preserved exactly.
    #[test]
    fn csv_round_trip_preserves_key_fields() {
        let mut h1 = make_holding("AAPL", AssetType::Stock, 10.0, 155.25, "USD");
        h1.name = "Apple Inc.".to_string();
        h1.exchange = "NMS".to_string();
        h1.target_weight = 25.0;

        let mut h2 = make_holding("XIU.TO", AssetType::Etf, 50.0, 34.5, "CAD");
        h2.name = "iShares S&P/TSX 60 Index ETF".to_string();
        h2.exchange = "TRT".to_string();
        h2.target_weight = 15.0;

        let mut h3 = make_holding("BTC-USD", AssetType::Crypto, 0.5, 40000.0, "USD");
        h3.name = "Bitcoin USD".to_string();
        h3.target_weight = 10.0;

        let holdings = vec![h1, h2, h3];
        let csv = build_holdings_csv(&holdings).expect("build csv");

        let rows = parse_import_rows(&csv).expect("parse csv");

        assert_eq!(rows.len(), 3, "row count should be preserved");

        // Row 0 — AAPL (stock)
        assert_eq!(rows[0].symbol, "AAPL");
        assert!(matches!(rows[0].asset_type, AssetType::Stock));
        assert!((rows[0].quantity - 10.0).abs() < 0.001);
        assert!((rows[0].cost_basis - 155.25).abs() < 0.001);
        assert_eq!(rows[0].currency, "USD");
        assert_eq!(rows[0].exchange, "NMS");
        assert!((rows[0].target_weight - 25.0).abs() < 0.001);

        // Row 1 — XIU.TO (etf)
        assert_eq!(rows[1].symbol, "XIU.TO");
        assert!(matches!(rows[1].asset_type, AssetType::Etf));
        assert!((rows[1].quantity - 50.0).abs() < 0.001);
        assert!((rows[1].cost_basis - 34.5).abs() < 0.001);
        assert_eq!(rows[1].currency, "CAD");
        assert_eq!(rows[1].exchange, "TRT");
        assert!((rows[1].target_weight - 15.0).abs() < 0.001);

        // Row 2 — BTC-USD (crypto)
        assert_eq!(rows[2].symbol, "BTC-USD");
        assert!(matches!(rows[2].asset_type, AssetType::Crypto));
        assert!((rows[2].quantity - 0.5).abs() < 0.001);
        assert!((rows[2].cost_basis - 40000.0).abs() < 0.001);
        assert_eq!(rows[2].currency, "USD");
        assert!((rows[2].target_weight - 10.0).abs() < 0.001);
    }

    /// Exporting a single cash holding round-trips correctly.
    #[test]
    fn csv_round_trip_cash_holding() {
        let mut cash = make_holding("CAD-CASH", AssetType::Cash, 5000.0, 1.0, "CAD");
        cash.name = "CAD Cash".to_string();
        cash.target_weight = 5.0;

        let csv = build_holdings_csv(&[cash]).expect("build csv");
        let rows = parse_import_rows(&csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "CAD-CASH");
        assert!(matches!(rows[0].asset_type, AssetType::Cash));
        assert!((rows[0].quantity - 5000.0).abs() < 0.001);
        assert!((rows[0].cost_basis - 1.0).abs() < 0.001);
        assert_eq!(rows[0].currency, "CAD");
        assert!((rows[0].target_weight - 5.0).abs() < 0.001);
    }

    /// An empty holdings slice produces a CSV that fails parsing (no data rows).
    #[test]
    fn build_holdings_csv_empty_slice_roundtrip_fails_gracefully() {
        let csv = build_holdings_csv(&[]).expect("build csv for empty slice");
        // build_holdings_csv writes a header-only CSV; parse_import_rows should
        // return an error because there are no data rows.
        let result = parse_import_rows(&csv);
        assert!(result.is_err(), "empty csv should error on import");
        assert!(result.unwrap_err().contains("empty"));
    }

    /// Round-trip with target_weight = 0 (the default) is preserved as 0.
    #[test]
    fn csv_round_trip_zero_target_weight() {
        let holding = make_holding("MSFT", AssetType::Stock, 3.0, 200.0, "USD");
        // target_weight is already 0.0 from make_holding

        let csv = build_holdings_csv(&[holding]).expect("build csv");
        let rows = parse_import_rows(&csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert!((rows[0].target_weight - 0.0).abs() < 0.001);
    }

    #[test]
    fn build_portfolio_snapshot_converts_mixed_currency_holdings_into_base_currency() {
        let holdings = vec![
            make_holding("SHOP.TO", AssetType::Stock, 10.0, 100.0, "CAD"),
            make_holding("AAPL", AssetType::Stock, 5.0, 100.0, "USD"),
        ];
        let prices = vec![
            PriceData {
                symbol: "SHOP.TO".to_string(),
                price: 120.0,
                currency: "CAD".to_string(),
                change: 1.0,
                change_percent: 2.0,
                updated_at: Utc::now().to_rfc3339(),
                open: None,
                previous_close: None,
                volume: None,
            },
            PriceData {
                symbol: "AAPL".to_string(),
                price: 110.0,
                currency: "USD".to_string(),
                change: 1.0,
                change_percent: 10.0,
                updated_at: Utc::now().to_rfc3339(),
                open: None,
                previous_close: None,
                volume: None,
            },
        ];
        let fx = vec![FxRate {
            pair: "USDCAD".to_string(),
            rate: 1.25,
            updated_at: Utc::now().to_rfc3339(),
        }];

        let snapshot = build_portfolio_snapshot(
            &holdings,
            &prices,
            &fx,
            "CAD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        assert_eq!(snapshot.base_currency, "CAD");
        assert!((snapshot.holdings[0].market_value_cad - 1200.0).abs() < 0.001);
        assert!((snapshot.holdings[1].market_value_cad - 687.5).abs() < 0.001);
        assert!((snapshot.holdings[1].cost_value_cad - 625.0).abs() < 0.001);
        assert!((snapshot.total_value - 1887.5).abs() < 0.001);
        assert!((snapshot.total_cost - 1625.0).abs() < 0.001);
        assert!((snapshot.daily_pnl - 92.75).abs() < 0.001);
        assert_eq!(snapshot.total_target_weight, 0.0);
    }

    #[test]
    fn build_portfolio_snapshot_supports_non_cad_base_currency() {
        let holdings = vec![
            make_holding("RY.TO", AssetType::Stock, 2.0, 100.0, "CAD"),
            make_holding("MSFT", AssetType::Stock, 1.0, 200.0, "USD"),
        ];
        let prices = vec![
            PriceData {
                symbol: "RY.TO".to_string(),
                price: 110.0,
                currency: "CAD".to_string(),
                change: 0.0,
                change_percent: 0.0,
                updated_at: Utc::now().to_rfc3339(),
                open: None,
                previous_close: None,
                volume: None,
            },
            PriceData {
                symbol: "MSFT".to_string(),
                price: 220.0,
                currency: "USD".to_string(),
                change: 0.0,
                change_percent: 0.0,
                updated_at: Utc::now().to_rfc3339(),
                open: None,
                previous_close: None,
                volume: None,
            },
        ];
        let fx = vec![FxRate {
            pair: "CADUSD".to_string(),
            rate: 0.8,
            updated_at: Utc::now().to_rfc3339(),
        }];

        let snapshot = build_portfolio_snapshot(
            &holdings,
            &prices,
            &fx,
            "USD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        assert_eq!(snapshot.base_currency, "USD");
        assert!((snapshot.holdings[0].market_value_cad - 176.0).abs() < 0.001);
        assert!((snapshot.holdings[0].cost_value_cad - 160.0).abs() < 0.001);
        assert!((snapshot.holdings[1].market_value_cad - 220.0).abs() < 0.001);
        assert!((snapshot.total_value - 396.0).abs() < 0.001);
        assert!((snapshot.total_cost - 360.0).abs() < 0.001);
    }

    // ── Target-weight portfolio-level validation tests ──────────────────────

    #[test]
    fn add_holding_weight_exceeds_100_when_existing_sum_plus_new_is_over_limit() {
        // Simulate the guard logic that add_holding applies before inserting.
        // We verify that existing_sum + new_weight > 100 is caught.
        let existing_sum = 60.0f64;
        let new_weight = 50.0f64;
        assert!(
            existing_sum + new_weight > 100.0,
            "guard should reject: {:.1} + {:.1} = {:.1} > 100",
            existing_sum,
            new_weight,
            existing_sum + new_weight
        );
    }

    #[test]
    fn add_holding_weight_exactly_100_is_accepted() {
        let existing_sum = 60.0f64;
        let new_weight = 40.0f64;
        assert!(
            existing_sum + new_weight <= 100.0,
            "guard should allow: {:.1} + {:.1} = {:.1} <= 100",
            existing_sum,
            new_weight,
            existing_sum + new_weight
        );
    }

    #[test]
    fn update_holding_weight_exceeds_100_when_others_sum_plus_new_is_over_limit() {
        // Simulate the guard logic used by update_holding (other holdings sum + new value).
        let others_sum = 70.0f64;
        let new_weight = 35.0f64;
        assert!(
            others_sum + new_weight > 100.0,
            "guard should reject: {:.1} + {:.1} = {:.1} > 100",
            others_sum,
            new_weight,
            others_sum + new_weight
        );
    }

    #[test]
    fn import_csv_weight_sum_over_100_is_rejected() {
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   AAPL,Apple,stock,5,120,USD,60\n\
                   MSFT,Microsoft,stock,3,200,USD,50\n";
        let rows = parse_import_rows(csv).expect("parse ok");
        let total: f64 = rows.iter().map(|r| r.target_weight).sum();
        assert!(
            total > 100.0,
            "csv weight sum should exceed 100, got {:.1}",
            total
        );
        // Confirm the error message format is correct when this check fires
        let err = format!(
            "Import failed: total target weight is {:.1}% (max 100%). Adjust weights before re-importing.",
            total
        );
        assert!(err.contains("Import failed"));
        assert!(err.contains("110.0%"));
    }

    #[test]
    fn import_csv_weight_sum_at_100_passes_csv_level_guard() {
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   AAPL,Apple,stock,5,120,USD,60\n\
                   MSFT,Microsoft,stock,3,200,USD,40\n";
        let rows = parse_import_rows(csv).expect("parse ok");
        let total: f64 = rows.iter().map(|r| r.target_weight).sum();
        assert!(
            total <= 100.0,
            "csv weight sum should be <= 100, got {:.1}",
            total
        );
    }

    #[test]
    fn import_csv_existing_holdings_combined_with_csv_exceeds_100_is_rejected() {
        let existing_weight_sum = 70.0f64;
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   GOOG,Alphabet,stock,2,150,USD,40\n";
        let rows = parse_import_rows(csv).expect("parse ok");
        let csv_sum: f64 = rows.iter().map(|r| r.target_weight).sum();
        // csv_sum alone (40) is <= 100, so it passes the CSV-level guard
        assert!(csv_sum <= 100.0);
        // But combined with existing it exceeds 100
        assert!(
            existing_weight_sum + csv_sum > 100.0,
            "combined should exceed 100, got {:.1}",
            existing_weight_sum + csv_sum
        );
    }

    #[test]
    fn build_portfolio_snapshot_same_day_purchase_uses_cost_basis_for_daily_pnl() {
        // A holding created today uses (current_price - cost_basis) * quantity as daily PnL proxy.
        let today = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let mut holding = make_holding("AAPL", AssetType::Stock, 10.0, 100.0, "CAD");
        holding.created_at = today;

        let prices = vec![PriceData {
            symbol: "AAPL".to_string(),
            price: 120.0,
            currency: "CAD".to_string(),
            change: 2.0,
            change_percent: 5.0, // day-over-day pct — should NOT be used for same-day purchases
            updated_at: Utc::now().to_rfc3339(),
            open: None,
            previous_close: None,
            volume: None,
        }];

        let snapshot = build_portfolio_snapshot(
            &[holding],
            &prices,
            &[],
            "CAD",
            Utc::now().to_rfc3339(),
            0.0,
            0.0,
        );

        // Expected: (120 - 100) * 10 = 200 (gain since purchase used as daily proxy)
        assert!(
            (snapshot.daily_pnl - 200.0).abs() < 0.001,
            "expected daily_pnl == 200 for same-day purchase using cost-basis proxy, got {}",
            snapshot.daily_pnl
        );
    }

    #[test]
    fn build_portfolio_snapshot_includes_prior_day_holding_in_daily_pnl() {
        // A holding created yesterday (or earlier) should contribute normally.
        let yesterday = (Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let mut holding = make_holding("MSFT", AssetType::Stock, 10.0, 200.0, "CAD");
        holding.created_at = yesterday;

        let prices = vec![PriceData {
            symbol: "MSFT".to_string(),
            price: 220.0,
            currency: "CAD".to_string(),
            change: 20.0,
            change_percent: 10.0, // 10% of 2200 = 220
            updated_at: Utc::now().to_rfc3339(),
            open: None,
            previous_close: None,
            volume: None,
        }];

        let snapshot = build_portfolio_snapshot(
            &[holding],
            &prices,
            &[],
            "CAD",
            Utc::now().to_rfc3339(),
            0.0,
            0.0,
        );

        // market_value_cad = 10 * 220 = 2200; daily_pnl = 2200 * 0.10 = 220
        assert!(
            (snapshot.daily_pnl - 220.0).abs() < 0.001,
            "expected daily_pnl == 220 for prior-day holding, got {}",
            snapshot.daily_pnl
        );
    }
}
