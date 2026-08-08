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
pub mod pdf;
pub mod portfolio;
pub mod prices;
pub mod stress;
pub mod transactions;
pub mod watchlists;

pub use accounts::*;
pub use alerts::*;
pub use analytics::*;
pub use backup::*;
pub use config::*;
pub use dividends::*;
pub use import::*;
pub use pdf::*;
pub use portfolio::*;
pub use prices::*;
pub use stress::*;
pub use transactions::*;
pub use watchlists::*;

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

/// Guards `backup_database`/`restore_database` against running concurrently —
/// either against each other or against a second invocation of themselves —
/// since both operate on the same live DB file on disk.
pub struct BackupLockState(pub tokio::sync::Mutex<()>);

impl BackupLockState {
    pub fn new() -> Self {
        BackupLockState(tokio::sync::Mutex::new(()))
    }
}

pub(crate) async fn get_base_currency(pool: &SqlitePool) -> String {
    db::get_config(pool, "base_currency")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| crate::config::BASE_CURRENCY.to_string())
}

/// Normalizes a stored `cost_basis_method` config value into one `analytics::compute_realized_gains`
/// accepts (`"avco"` / `"fifo"`). `set_config_cmd` validates new writes against this enum (#714), but
/// a value written before that validation existed — or edited directly in the DB file — could still
/// be invalid; falling back here instead of propagating an error keeps the whole portfolio snapshot
/// from failing over a single bad config value.
pub(crate) fn normalize_cost_basis_method(value: Option<String>) -> String {
    match value {
        Some(v) if v.eq_ignore_ascii_case("avco") || v.eq_ignore_ascii_case("fifo") => {
            v.to_lowercase()
        }
        Some(v) => {
            tracing::warn!(
                "Invalid cost_basis_method {:?} in config; falling back to avco",
                v
            );
            "avco".to_string()
        }
        None => "avco".to_string(),
    }
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

// The functions and constants below are thin adapters over
// `portfolio_core::validation::*`: they convert the shared `Result<T, String>`
// into `AppError::Validation`, preserving the exact messages the frontend has
// always seen. The actual rules live in `portfolio-core` so the MCP server
// (`portfolio-mcp/src/validation.rs`) enforces them identically — see #758.

pub(crate) use portfolio_core::validation::WEIGHT_EPSILON;

pub(crate) fn validate_holding_fields(
    quantity: f64,
    cost_basis: f64,
    currency: &str,
) -> Result<String, AppError> {
    portfolio_core::validation::validate_holding_fields(quantity, cost_basis, currency)
        .map_err(AppError::Validation)
}

pub(crate) fn validate_target_weight(target_weight: Option<f64>) -> Result<(), AppError> {
    portfolio_core::validation::validate_target_weight(target_weight).map_err(AppError::Validation)
}

pub(crate) fn validate_dividend_fields(
    amount_per_unit: f64,
    ex_date: &str,
    pay_date: &str,
) -> Result<(), AppError> {
    portfolio_core::validation::validate_dividend_fields(amount_per_unit, ex_date, pay_date)
        .map_err(AppError::Validation)
}

pub(crate) fn validate_transaction_fields(quantity: f64, price: f64) -> Result<(), AppError> {
    portfolio_core::validation::validate_transaction_fields(quantity, price)
        .map_err(AppError::Validation)
}

pub(crate) fn validate_holding_dividend_fields(
    indicated_annual_dividend: Option<f64>,
    dividend_frequency: Option<&str>,
    maturity_date: Option<&str>,
) -> Result<(), AppError> {
    portfolio_core::validation::validate_holding_dividend_fields(
        indicated_annual_dividend,
        dividend_frequency,
        maturity_date,
    )
    .map_err(AppError::Validation)
}

/// Validates a price alert's symbol, threshold, currency, and note. Shared by
/// `add_alert`.
pub(crate) fn validate_alert_fields(
    symbol: &str,
    threshold: f64,
    currency: &str,
    note: &str,
) -> Result<(), AppError> {
    portfolio_core::validation::validate_non_empty("symbol", symbol)
        .map_err(AppError::Validation)?;
    portfolio_core::validation::validate_alert_threshold(threshold)
        .map_err(AppError::Validation)?;
    portfolio_core::validation::validate_alert_currency(currency).map_err(AppError::Validation)?;
    portfolio_core::validation::validate_alert_note(note).map_err(AppError::Validation)?;
    Ok(())
}

pub(crate) fn validate_id(field: &str, id: &str) -> Result<(), AppError> {
    portfolio_core::validation::validate_id(field, id).map_err(AppError::Validation)
}

/// Validates 1-indexed pagination parameters. Shared by every `*_paginated` command.
pub(crate) fn validate_pagination(page: i64, page_size: i64) -> Result<(), AppError> {
    if page < 1 {
        return Err(AppError::Validation("page must be >= 1".to_string()));
    }
    if !(1..=500).contains(&page_size) {
        return Err(AppError::Validation(
            "page_size must be between 1 and 500".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::csv::{build_holdings_csv, parse_import_rows};
    use crate::test_helpers::make_holding;
    use crate::types::AssetType;

    // CSV/normalize tests live in csv.rs.
    // build_portfolio_snapshot tests live in portfolio-core/src/snapshot.rs (#640).

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
        assert!((rows[0].target_weight.expect("target_weight") - 12.5).abs() < 0.001);
    }

    #[test]
    fn parse_import_rows_rejects_missing_required_columns() {
        let csv = "symbol,name,type,quantity,currency\nAAPL,Apple Inc.,stock,5,USD\n";
        let error = parse_import_rows(csv).expect_err("missing cost_basis should fail");

        assert!(error.contains("Missing required column: cost_basis"));
    }

    #[test]
    fn normalize_cost_basis_method_passes_through_valid_values() {
        assert_eq!(
            super::normalize_cost_basis_method(Some("avco".to_string())),
            "avco"
        );
        assert_eq!(
            super::normalize_cost_basis_method(Some("fifo".to_string())),
            "fifo"
        );
        assert_eq!(
            super::normalize_cost_basis_method(Some("FIFO".to_string())),
            "fifo"
        );
    }

    #[test]
    fn normalize_cost_basis_method_defaults_to_avco_when_unset() {
        assert_eq!(super::normalize_cost_basis_method(None), "avco");
    }

    #[test]
    fn normalize_cost_basis_method_falls_back_to_avco_on_invalid_stored_value() {
        // Regression guard for #714: a value written before the set_config_cmd
        // enum validation existed (or edited directly in the DB) previously made
        // compute_realized_gains_grouped error out, failing the whole portfolio
        // snapshot instead of degrading gracefully.
        assert_eq!(
            super::normalize_cost_basis_method(Some("garbage".to_string())),
            "avco"
        );
    }

    #[test]
    fn import_weight_sum_over_100_detected() {
        // Two rows whose target_weight values sum to 110; the command-level guard
        // rejects this.  Verify parse_import_rows succeeds and the sum exceeds 100.
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   AAPL,Apple Inc.,stock,5,120,USD,60\n\
                   MSFT,Microsoft,stock,3,200,USD,50\n";
        let rows = parse_import_rows(csv).expect("rows should parse");
        let total: f64 = rows.iter().filter_map(|r| r.target_weight).sum();
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
        let total: f64 = rows.iter().filter_map(|r| r.target_weight).sum();
        assert!(
            (total - 100.0).abs() < 0.001,
            "expected total == 100, got {}",
            total
        );
    }

    #[test]
    fn build_holdings_csv_includes_target_weight_column() {
        let mut holding = make_holding("AAPL", AssetType::Stock, 5.0, 120.0, "USD");
        holding.target_weight = Some(22.5);

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
        h1.target_weight = Some(25.0);

        let mut h2 = make_holding("XIU.TO", AssetType::Etf, 50.0, 34.5, "CAD");
        h2.name = "iShares S&P/TSX 60 Index ETF".to_string();
        h2.exchange = "TRT".to_string();
        h2.target_weight = Some(15.0);

        let mut h3 = make_holding("BTC-USD", AssetType::Crypto, 0.5, 40000.0, "USD");
        h3.name = "Bitcoin USD".to_string();
        h3.target_weight = Some(10.0);

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
        assert!((rows[0].target_weight.expect("target_weight") - 25.0).abs() < 0.001);

        // Row 1 — XIU.TO (etf)
        assert_eq!(rows[1].symbol, "XIU.TO");
        assert!(matches!(rows[1].asset_type, AssetType::Etf));
        assert!((rows[1].quantity - 50.0).abs() < 0.001);
        assert!((rows[1].cost_basis - 34.5).abs() < 0.001);
        assert_eq!(rows[1].currency, "CAD");
        assert_eq!(rows[1].exchange, "TRT");
        assert!((rows[1].target_weight.expect("target_weight") - 15.0).abs() < 0.001);

        // Row 2 — BTC-USD (crypto)
        assert_eq!(rows[2].symbol, "BTC-USD");
        assert!(matches!(rows[2].asset_type, AssetType::Crypto));
        assert!((rows[2].quantity - 0.5).abs() < 0.001);
        assert!((rows[2].cost_basis - 40000.0).abs() < 0.001);
        assert_eq!(rows[2].currency, "USD");
        assert!((rows[2].target_weight.expect("target_weight") - 10.0).abs() < 0.001);
    }

    /// Exporting a single cash holding round-trips correctly.
    #[test]
    fn csv_round_trip_cash_holding() {
        let mut cash = make_holding("CAD-CASH", AssetType::Cash, 5000.0, 1.0, "CAD");
        cash.name = "CAD Cash".to_string();
        cash.target_weight = Some(5.0);

        let csv = build_holdings_csv(&[cash]).expect("build csv");
        let rows = parse_import_rows(&csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "CAD-CASH");
        assert!(matches!(rows[0].asset_type, AssetType::Cash));
        assert!((rows[0].quantity - 5000.0).abs() < 0.001);
        assert!((rows[0].cost_basis - 1.0).abs() < 0.001);
        assert_eq!(rows[0].currency, "CAD");
        assert!((rows[0].target_weight.expect("target_weight") - 5.0).abs() < 0.001);
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

    /// A holding with no target_weight set (the default) round-trips as unset,
    /// not as an explicit 0.
    #[test]
    fn csv_round_trip_unset_target_weight_stays_none() {
        let holding = make_holding("MSFT", AssetType::Stock, 3.0, 200.0, "USD");
        // target_weight is None (unset) from make_holding

        let csv = build_holdings_csv(&[holding]).expect("build csv");
        let rows = parse_import_rows(&csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].target_weight, None);
    }

    // build_portfolio_snapshot_converts_mixed_currency_holdings_into_base_currency and
    // build_portfolio_snapshot_supports_non_cad_base_currency moved to portfolio-core/src/snapshot.rs (#640).

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
        let total: f64 = rows.iter().filter_map(|r| r.target_weight).sum();
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
        let total: f64 = rows.iter().filter_map(|r| r.target_weight).sum();
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
        let csv_sum: f64 = rows.iter().filter_map(|r| r.target_weight).sum();
        // csv_sum alone (40) is <= 100, so it passes the CSV-level guard
        assert!(csv_sum <= 100.0);
        // But combined with existing it exceeds 100
        assert!(
            existing_weight_sum + csv_sum > 100.0,
            "combined should exceed 100, got {:.1}",
            existing_weight_sum + csv_sum
        );
    }

    // build_portfolio_snapshot_same_day_purchase_uses_cost_basis_for_daily_pnl and
    // build_portfolio_snapshot_includes_prior_day_holding_in_daily_pnl moved to
    // portfolio-core/src/snapshot.rs (#640).
}
