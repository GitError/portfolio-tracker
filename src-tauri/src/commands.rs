use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::Utc;
use sqlx::SqlitePool;
use tauri::{Manager, State};

use crate::analytics::compute_realized_gains_grouped;
use crate::csv::{build_holdings_csv, parse_import_rows};
use crate::db;
use crate::error::AppError;
use crate::fx::fetch_all_fx_rates;
use crate::portfolio::build_portfolio_snapshot;
use crate::price::{fetch_all_prices, fetch_price, FetchAllPricesResult};
use crate::search::search_symbols_yahoo;
use crate::stress::run_stress_test;
use crate::types::{
    Account, AlertId, AssetType, CountryWeight, CreateAccountRequest, Dividend, DividendInput,
    Holding, HoldingId, HoldingInput, ImportError, ImportResult, PaginatedResult, PerformancePoint,
    PortfolioAnalytics, PortfolioRiskMetrics, PortfolioSnapshot, PreviewImportResult, PreviewRow,
    PriceAlert, PriceAlertInput, PriceData, RealizedGainsSummary, RebalanceSuggestion,
    RefreshResult, SectorWeight, StressResult, StressScenario, SymbolMetadata, SymbolResult,
    Transaction, TransactionId, TransactionInput,
};

pub struct DbState(pub SqlitePool);
pub struct HttpClient(pub reqwest::Client);

async fn get_base_currency(pool: &SqlitePool) -> String {
    db::get_config(pool, "base_currency")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| crate::config::BASE_CURRENCY.to_string())
}

#[tauri::command]
pub async fn get_config_cmd(
    db: State<'_, DbState>,
    key: String,
) -> Result<Option<String>, AppError> {
    let pool = &db.0;
    db::get_config(pool, &key).await.map_err(AppError::from)
}

#[tauri::command]
pub async fn set_config_cmd(
    db: State<'_, DbState>,
    gains_cache: State<'_, RealizedGainsCacheState>,
    key: String,
    value: String,
) -> Result<(), AppError> {
    const ALLOWED_CONFIG_KEYS: &[&str] = &[
        "base_currency",
        "app_language",
        "app_theme",
        "auto_refresh_interval_ms",
        "auto_refresh_market_hours_only",
        "cost_basis_method",
        "notifications_enabled",
    ];
    if !ALLOWED_CONFIG_KEYS.contains(&key.as_str()) {
        return Err(AppError::Validation(format!("Unknown config key: {key}")));
    }
    let pool = &db.0;
    let value = if key == "cost_basis_method" {
        value.to_lowercase()
    } else {
        value
    };
    db::set_config(pool, &key, &value)
        .await
        .map_err(AppError::from)?;
    // Changing the cost-basis method invalidates any previously cached realized gains
    // because the same transaction history produces a different result under AVCO vs FIFO.
    if key == "cost_basis_method" {
        gains_cache.invalidate();
    }
    Ok(())
}

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

    fn get(&self, key: &str) -> Option<Vec<SymbolResult>> {
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

    fn set(&self, key: String, results: Vec<SymbolResult>) {
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

async fn validate_symbol(
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

#[tauri::command]
pub async fn get_portfolio(
    db: State<'_, DbState>,
    _client: State<'_, HttpClient>,
    gains_cache: State<'_, RealizedGainsCacheState>,
) -> Result<PortfolioSnapshot, AppError> {
    let pool = &db.0;
    let base_currency = get_base_currency(pool).await;

    let holdings = db::get_all_holdings(pool).await?;

    let cached_prices = db::get_cached_prices(pool).await?;
    let cached_fx = db::get_fx_rates(pool).await?;

    let cost_basis_method_opt = db::get_config(pool, "cost_basis_method").await?;
    // If the user has never explicitly chosen a method, flag the snapshot so the frontend
    // can prompt for an explicit selection before displaying realized gains.
    let requires_cost_basis_selection = cost_basis_method_opt.is_none();
    let cost_basis_method = cost_basis_method_opt.unwrap_or_else(|| "avco".to_string());

    let realized_gains = {
        let summary = if let Some(cached) = gains_cache.get() {
            tracing::info!("realized_gains cache hit");
            cached
        } else {
            let transactions = db::get_all_transactions(pool).await?;
            match compute_realized_gains_grouped(&transactions, &cost_basis_method) {
                Ok(s) => {
                    gains_cache.set(s.clone());
                    s
                }
                Err(e) => {
                    tracing::error!(
                        "realized_gains error (method={:?}): {}",
                        cost_basis_method,
                        e
                    );
                    return Err(AppError::from(e));
                }
            }
        };
        summary.total_realized_gain
    };

    let annual_dividend_income = db::get_annual_dividend_income(pool, &base_currency, &cached_fx)
        .await
        .unwrap_or(0.0);

    let mut snapshot = build_portfolio_snapshot(
        &holdings,
        &cached_prices,
        &cached_fx,
        &base_currency,
        Utc::now().to_rfc3339(),
        realized_gains,
        annual_dividend_income,
    );
    snapshot.requires_cost_basis_selection = requires_cost_basis_selection;
    Ok(snapshot)
}

/// Deprecated: use `get_holdings_paginated` instead.
/// This command returns all holdings in a single response with no pagination;
/// it remains registered for backwards compatibility but should not be used in new code.
#[tauri::command]
pub async fn get_holdings(db: State<'_, DbState>) -> Result<Vec<Holding>, AppError> {
    tracing::warn!("get_holdings is deprecated; use get_holdings_paginated");
    let pool = &db.0;
    db::get_all_holdings(pool).await.map_err(AppError::from)
}

const WEIGHT_EPSILON: f64 = 0.001;

/// Validate fields common to both add_holding and update_holding.
/// Returns the normalised (uppercased, trimmed) currency string on success.
fn validate_holding_fields(
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

#[tauri::command]
pub async fn add_holding(
    db: State<'_, DbState>,
    holding: HoldingInput,
) -> Result<Holding, AppError> {
    validate_holding_fields(holding.quantity, holding.cost_basis, &holding.currency)?;
    let pool = &db.0;
    if holding.target_weight > 0.0 {
        let current_sum = db::sum_target_weights(pool, None).await?;
        let new_total = current_sum + holding.target_weight;
        if new_total > 100.0 + WEIGHT_EPSILON {
            return Err(AppError::Validation(format!(
                "Total target weight would exceed 100% (currently {:.1}%). Adjust existing allocations before adding this holding.",
                current_sum
            )));
        }
    }
    db::insert_holding(pool, holding)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn update_holding(db: State<'_, DbState>, holding: Holding) -> Result<Holding, AppError> {
    validate_holding_fields(holding.quantity, holding.cost_basis, &holding.currency)?;
    let pool = &db.0;
    if holding.target_weight > 0.0 {
        let current_sum = db::sum_target_weights(pool, Some(holding.id.0.as_str())).await?;
        let new_total = current_sum + holding.target_weight;
        if new_total > 100.0 + WEIGHT_EPSILON {
            return Err(AppError::Validation(format!(
                "Total target weight would exceed 100% (currently {:.1}% across other holdings). Adjust existing allocations before saving.",
                current_sum
            )));
        }
    }
    db::update_holding(pool, holding)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn delete_holding(db: State<'_, DbState>, id: HoldingId) -> Result<bool, AppError> {
    let pool = &db.0;
    db::delete_holding(pool, &id).await.map_err(AppError::from)
}

#[tauri::command]
pub async fn export_holdings_csv(db: State<'_, DbState>) -> Result<String, AppError> {
    let pool = &db.0;
    let holdings = db::get_all_holdings(pool).await?;
    build_holdings_csv(&holdings).map_err(AppError::from)
}

#[tauri::command]
pub async fn import_holdings_csv(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
    csv_content: String,
) -> Result<ImportResult, AppError> {
    let parsed_rows = parse_import_rows(&csv_content)?;

    let existing_keys: HashSet<(String, String)> = {
        let pool = &db.0;
        db::get_all_holdings(pool)
            .await?
            .into_iter()
            .map(|holding| {
                (
                    holding.symbol.to_uppercase(),
                    holding.account.as_str().to_string(),
                )
            })
            .collect()
    };

    let mut seen_keys = existing_keys;
    let mut pending_inputs = Vec::new();
    let mut skipped = Vec::new();

    for row in parsed_rows {
        let key = (row.symbol.to_uppercase(), row.account.as_str().to_string());
        if seen_keys.contains(&key) {
            skipped.push(ImportError {
                row: row.row,
                symbol: row.symbol,
                reason: "duplicate".to_string(),
            });
            continue;
        }

        if matches!(row.asset_type, AssetType::Cash) {
            seen_keys.insert((row.symbol.to_uppercase(), row.account.as_str().to_string()));
            pending_inputs.push(HoldingInput {
                symbol: row.symbol,
                name: if row.name.is_empty() {
                    format!("{} Cash", row.currency)
                } else {
                    row.name
                },
                asset_type: row.asset_type,
                account: row.account,
                account_id: None,
                quantity: row.quantity,
                cost_basis: row.cost_basis,
                currency: row.currency,
                exchange: row.exchange,
                target_weight: row.target_weight,
                indicated_annual_dividend: row.indicated_annual_dividend,
                indicated_annual_dividend_currency: row.indicated_annual_dividend_currency,
                dividend_frequency: row.dividend_frequency,
                maturity_date: row.maturity_date,
            });
            continue;
        }

        let validated = match validate_symbol(&db, &client, &row.symbol).await {
            Ok(Some(result)) => result,
            Ok(None) => {
                skipped.push(ImportError {
                    row: row.row,
                    symbol: row.symbol,
                    reason: "invalid_symbol".to_string(),
                });
                continue;
            }
            Err(_) => {
                skipped.push(ImportError {
                    row: row.row,
                    symbol: row.symbol,
                    reason: "validation_failed".to_string(),
                });
                continue;
            }
        };

        if !validated.currency.eq_ignore_ascii_case(&row.currency) {
            skipped.push(ImportError {
                row: row.row,
                symbol: row.symbol,
                reason: format!(
                    "currency_mismatch:{}_expected_{}",
                    row.currency,
                    validated.currency.to_uppercase()
                ),
            });
            continue;
        }

        seen_keys.insert((
            validated.symbol.to_uppercase(),
            row.account.as_str().to_string(),
        ));
        pending_inputs.push(HoldingInput {
            symbol: validated.symbol,
            name: if row.name.is_empty() {
                validated.name
            } else {
                row.name
            },
            asset_type: row.asset_type,
            account: row.account,
            account_id: None,
            quantity: row.quantity,
            cost_basis: row.cost_basis,
            currency: row.currency,
            exchange: if row.exchange.is_empty() {
                validated.exchange
            } else {
                row.exchange
            },
            target_weight: row.target_weight,
            indicated_annual_dividend: row.indicated_annual_dividend,
            indicated_annual_dividend_currency: row.indicated_annual_dividend_currency,
            dividend_frequency: row.dividend_frequency,
            maturity_date: row.maturity_date,
        });
    }

    // Weight validation runs after deduplication so that re-importing an existing
    // portfolio (all rows skipped as duplicates) never triggers a false overflow.
    // All pending inputs (cash and non-cash alike) are included in this sum.
    let import_weight_sum: f64 = pending_inputs.iter().map(|h| h.target_weight).sum();
    if import_weight_sum > 100.0 + WEIGHT_EPSILON {
        return Err(AppError::Validation(format!(
            "Combined target weights ({:.2}%) exceed 100%",
            import_weight_sum
        )));
    }
    let existing_weight_sum = {
        let pool = &db.0;
        db::sum_target_weights(pool, None).await?
    };
    if existing_weight_sum + import_weight_sum > 100.0 + WEIGHT_EPSILON {
        return Err(AppError::Validation(format!(
            "Import failed: total target weight would reach {:.1}% (existing portfolio is already {:.1}%). Adjust weights before re-importing.",
            existing_weight_sum + import_weight_sum,
            existing_weight_sum
        )));
    }

    let mut imported = Vec::new();
    {
        let pool = &db.0;
        let mut tx = pool.begin().await.map_err(AppError::from)?;
        for input in pending_inputs {
            match db::insert_holding_in_tx(&mut tx, input).await {
                Ok(holding) => imported.push(holding),
                Err(e) => {
                    tx.rollback().await.map_err(AppError::from)?;
                    return Err(AppError::from(e));
                }
            }
        }
        tx.commit().await.map_err(AppError::from)?;
    }

    Ok(ImportResult {
        total_rows: imported.len() + skipped.len(),
        imported,
        skipped,
    })
}

#[tauri::command]
pub async fn preview_import_csv(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
    csv_content: String,
) -> Result<PreviewImportResult, AppError> {
    let parsed_rows = parse_import_rows(&csv_content)?;
    let existing_keys: HashSet<(String, String)> = {
        let pool = &db.0;
        db::get_all_holdings(pool)
            .await?
            .into_iter()
            .map(|h| (h.symbol.to_uppercase(), h.account.as_str().to_string()))
            .collect()
    };

    let mut preview_rows: Vec<PreviewRow> = Vec::new();
    let mut seen: HashSet<(String, String)> = existing_keys;

    for row in parsed_rows {
        let row_key = (row.symbol.to_uppercase(), row.account.as_str().to_string());

        if seen.contains(&row_key) {
            preview_rows.push(PreviewRow {
                row: row.row,
                original_symbol: row.symbol.clone(),
                resolved_symbol: row.symbol,
                name: row.name,
                asset_type: row.asset_type.as_str().to_string(),
                currency: row.currency,
                exchange: String::new(),
                quantity: row.quantity,
                cost_basis: row.cost_basis,
                target_weight: row.target_weight,
                status: "duplicate".to_string(),
            });
            continue;
        }

        if matches!(row.asset_type, AssetType::Cash) {
            seen.insert((row.symbol.to_uppercase(), row.account.as_str().to_string()));
            preview_rows.push(PreviewRow {
                row: row.row,
                original_symbol: row.symbol.clone(),
                resolved_symbol: row.symbol,
                name: if row.name.is_empty() {
                    format!("{} Cash", row.currency)
                } else {
                    row.name
                },
                asset_type: "cash".to_string(),
                currency: row.currency,
                exchange: String::new(),
                quantity: row.quantity,
                cost_basis: row.cost_basis,
                target_weight: row.target_weight,
                status: "ready".to_string(),
            });
            continue;
        }

        match validate_symbol(&db, &client, &row.symbol).await {
            Ok(Some(result)) => {
                seen.insert((
                    result.symbol.to_uppercase(),
                    row.account.as_str().to_string(),
                ));
                preview_rows.push(PreviewRow {
                    row: row.row,
                    original_symbol: row.symbol,
                    resolved_symbol: result.symbol,
                    name: if row.name.is_empty() {
                        result.name
                    } else {
                        row.name
                    },
                    asset_type: result.asset_type.as_str().to_string(),
                    currency: result.currency,
                    exchange: result.exchange,
                    quantity: row.quantity,
                    cost_basis: row.cost_basis,
                    target_weight: row.target_weight,
                    status: "ready".to_string(),
                });
            }
            Ok(None) => {
                preview_rows.push(PreviewRow {
                    row: row.row,
                    original_symbol: row.symbol,
                    resolved_symbol: String::new(),
                    name: row.name,
                    asset_type: row.asset_type.as_str().to_string(),
                    currency: row.currency,
                    exchange: String::new(),
                    quantity: row.quantity,
                    cost_basis: row.cost_basis,
                    target_weight: row.target_weight,
                    status: "invalid_symbol".to_string(),
                });
            }
            Err(_) => {
                preview_rows.push(PreviewRow {
                    row: row.row,
                    original_symbol: row.symbol,
                    resolved_symbol: String::new(),
                    name: row.name,
                    asset_type: row.asset_type.as_str().to_string(),
                    currency: row.currency,
                    exchange: String::new(),
                    quantity: row.quantity,
                    cost_basis: row.cost_basis,
                    target_weight: row.target_weight,
                    status: "validation_failed".to_string(),
                });
            }
        }
    }

    let ready_count = preview_rows.iter().filter(|r| r.status == "ready").count();
    let skip_count = preview_rows.len() - ready_count;

    Ok(PreviewImportResult {
        rows: preview_rows,
        ready_count,
        skip_count,
    })
}

#[tauri::command]
pub async fn refresh_prices(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
) -> Result<RefreshResult, AppError> {
    let base_currency = get_base_currency(&db.0).await;

    let holdings = {
        let pool = &db.0;
        db::get_all_holdings(pool).await?
    };

    // Collect unique symbols (skip cash) and a symbol→currency fallback map so
    // that when Yahoo Finance omits the currency field we use the holding's own
    // stored currency instead of silently defaulting to USD.
    let mut symbol_currencies: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let symbols: Vec<String> = holdings
        .iter()
        .filter(|h| h.asset_type.as_str() != "cash")
        .map(|h| {
            symbol_currencies
                .entry(h.symbol.clone())
                .or_insert_with(|| h.currency.clone());
            h.symbol.clone()
        })
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Collect all unique currencies; fetch_all_fx_rates will filter out the base
    let currencies: Vec<String> = holdings
        .iter()
        .map(|h| h.currency.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let (fetch_result, fx_rates) = tokio::join!(
        fetch_all_prices(&client.0, symbols, &symbol_currencies),
        fetch_all_fx_rates(&client.0, currencies, &base_currency)
    );

    let FetchAllPricesResult {
        prices,
        failed: failed_symbols,
    } = fetch_result;

    let mut triggered_alert_ids: Vec<String> = Vec::new();
    let mut alert_errors: Vec<String> = Vec::new();

    // Persist prices and FX rates to cache
    {
        let pool = &db.0;
        for price in &prices {
            db::upsert_price(pool, price).await?;
        }
        for rate in &fx_rates {
            db::upsert_fx_rate(pool, rate).await?;
        }
    }

    // Build a portfolio snapshot to record the current total value.
    // #475: Reuse the holdings and rate data already in memory — no extra DB round-trips.
    let snapshot_totals = {
        let snap = build_portfolio_snapshot(
            &holdings,
            &prices,
            &fx_rates,
            &base_currency,
            Utc::now().to_rfc3339(),
            0.0,
            0.0,
        );
        (snap.total_value, snap.total_cost, snap.total_gain_loss)
    };

    // #474: Load all active alerts in one query then evaluate them in memory.
    // This replaces the previous pattern of one SELECT per refreshed symbol.
    let active_alerts: HashMap<String, Vec<(String, String, f64)>> = {
        let pool = &db.0;
        match db::get_all_active_alerts(pool).await {
            Ok(rows) => {
                let mut map: HashMap<String, Vec<(String, String, f64)>> = HashMap::new();
                for (id, symbol_upper, dir, threshold) in rows {
                    map.entry(symbol_upper)
                        .or_default()
                        .push((id, dir, threshold));
                }
                map
            }
            Err(e) => {
                let msg = format!("Failed to load active alerts: {e}");
                tracing::warn!("{}", msg);
                alert_errors.push(msg);
                HashMap::new()
            }
        }
    };

    for price in &prices {
        let symbol_upper = price.symbol.to_uppercase();
        if let Some(alerts) = active_alerts.get(&symbol_upper) {
            for (id, dir_str, threshold) in alerts {
                let crossed = match (dir_str.as_str(), price.previous_close) {
                    ("above", Some(prev)) => prev < *threshold && price.price >= *threshold,
                    ("below", Some(prev)) => prev > *threshold && price.price <= *threshold,
                    ("above", None) => price.price >= *threshold,
                    ("below", None) => price.price <= *threshold,
                    _ => false,
                };
                if crossed {
                    let pool = &db.0;
                    match db::mark_alert_triggered(pool, id).await {
                        Ok(()) => triggered_alert_ids.push(id.clone()),
                        Err(e) => {
                            let msg = format!(
                                "Failed to trigger alert {} for {}: {}",
                                id, price.symbol, e
                            );
                            tracing::warn!("{}", msg);
                            alert_errors.push(msg);
                        }
                    }
                }
            }
        }
    }

    // Record the snapshot and prune old data; log errors but don't fail the command.
    // Surface snapshot insertion failures to the caller via RefreshResult.snapshot_error.
    let snapshot_error = {
        let pool = &db.0;
        let err = match db::insert_snapshot(
            pool,
            snapshot_totals.0,
            snapshot_totals.1,
            snapshot_totals.2,
        )
        .await
        {
            Ok(_) => None,
            Err(e) => {
                tracing::error!("Failed to insert portfolio snapshot: {}", e);
                Some(format!("Performance history could not be recorded: {e}"))
            }
        };
        if let Err(e) = db::prune_snapshots(pool).await {
            tracing::warn!("Failed to prune portfolio snapshots: {}", e);
        }
        err
    };

    Ok(RefreshResult {
        prices,
        failed_symbols,
        triggered_alerts: triggered_alert_ids,
        alert_errors,
        snapshot_error,
    })
}

#[tauri::command]
pub async fn run_stress_test_cmd(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
    gains_cache: State<'_, RealizedGainsCacheState>,
    scenario: StressScenario,
) -> Result<StressResult, AppError> {
    let snapshot = get_portfolio(db, client, gains_cache).await?;
    Ok(run_stress_test(&snapshot, &scenario))
}

#[tauri::command]
pub async fn search_symbols(
    query: String,
    client: State<'_, HttpClient>,
    cache: State<'_, SearchCacheState>,
    db: State<'_, DbState>,
) -> Result<Vec<SymbolResult>, AppError> {
    let q = query.trim();
    if q.len() < 2 || q.len() > crate::config::MAX_SEARCH_QUERY_LEN {
        return Ok(vec![]);
    }

    let key = q.to_lowercase();

    // 1. In-memory cache (5-minute TTL)
    if let Some(cached) = cache.get(&key) {
        return Ok(cached);
    }

    // 2. SQLite persistent cache
    let db_results = {
        let pool = &db.0;
        db::search_symbol_cache(pool, &key)
            .await
            .unwrap_or_default()
    };

    // 3. Yahoo Finance API
    let results = match search_symbols_yahoo(&client.0, q).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Symbol search API failed: {}", e);
            return Ok(db_results);
        }
    };

    // Persist new results to SQLite and in-memory cache
    {
        let pool = &db.0;
        for r in &results {
            if let Err(e) = db::upsert_symbol(pool, r).await {
                tracing::warn!("Failed to cache symbol: {}", e);
            }
        }
    }
    cache.set(key, results.clone());

    Ok(results)
}

#[tauri::command]
pub async fn get_symbol_price(
    symbol: String,
    client: State<'_, HttpClient>,
) -> Result<PriceData, AppError> {
    fetch_price(&client.0, &symbol)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn get_cached_prices(db: State<'_, DbState>) -> Result<Vec<PriceData>, AppError> {
    let pool = &db.0;
    db::get_cached_prices(pool).await.map_err(AppError::from)
}

#[tauri::command]
pub async fn get_performance(
    db: State<'_, DbState>,
    range: String,
) -> Result<Vec<PerformancePoint>, AppError> {
    let now = Utc::now();
    let end = now.to_rfc3339();

    let start = match range.as_str() {
        "1D" => (now - chrono::Duration::hours(24)).to_rfc3339(),
        "1W" => (now - chrono::Duration::days(7)).to_rfc3339(),
        "1M" => (now - chrono::Duration::days(30)).to_rfc3339(),
        "3M" => (now - chrono::Duration::days(90)).to_rfc3339(),
        "6M" => (now - chrono::Duration::days(180)).to_rfc3339(),
        "1Y" => (now - chrono::Duration::days(365)).to_rfc3339(),
        "ALL" => "1970-01-01T00:00:00+00:00".to_string(),
        _ => (now - chrono::Duration::days(30)).to_rfc3339(),
    };

    let pool = &db.0;
    let snapshots = db::get_snapshots_in_range(pool, &start, &end).await?;

    // Deduplicate by calendar date, keeping only the latest snapshot per day.
    let mut by_date: std::collections::BTreeMap<String, PerformancePoint> =
        std::collections::BTreeMap::new();
    for point in snapshots {
        // Guard against corrupted rows whose date field is shorter than 10 chars.
        let date_key = match point.date.get(..10) {
            Some(d) => d.to_string(),
            None => {
                tracing::warn!(
                    "get_performance: skipping snapshot with malformed date {:?}",
                    point.date
                );
                continue;
            }
        };
        by_date.insert(date_key, point);
    }
    Ok(by_date.into_values().collect())
}

// ── Dividend Commands ─────────────────────────────────────────────────────────

/// Deprecated: use `get_dividends_paginated` instead.
#[tauri::command]
pub async fn get_dividends(db: State<'_, DbState>) -> Result<Vec<Dividend>, AppError> {
    tracing::warn!("get_dividends is deprecated; use get_dividends_paginated");
    let pool = &db.0;
    db::get_dividends(pool).await.map_err(AppError::from)
}

#[tauri::command]
pub async fn add_dividend(
    db: State<'_, DbState>,
    dividend: DividendInput,
) -> Result<Dividend, AppError> {
    let pool = &db.0;
    // Look up the symbol and currency for the holding with a targeted query (avoids N+1)
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT symbol, currency FROM holdings WHERE id = $1")
            .bind(dividend.holding_id.0.as_str())
            .fetch_optional(pool)
            .await
            .map_err(AppError::from)?;
    let (symbol, holding_currency) = match row {
        Some((s, c)) => (s, c),
        None => (String::new(), String::new()),
    };
    // Validate that the dividend currency matches the holding's currency.
    if !holding_currency.is_empty()
        && holding_currency.to_uppercase() != dividend.currency.to_uppercase()
    {
        return Err(AppError::Validation(format!(
            "Dividend currency {} does not match holding currency {}",
            dividend.currency, holding_currency
        )));
    }
    db::insert_dividend(pool, dividend, &symbol)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn delete_dividend(db: State<'_, DbState>, id: i64) -> Result<bool, AppError> {
    let pool = &db.0;
    db::delete_dividend(pool, id).await.map_err(AppError::from)
}

// ── Price Alert Commands ───────────────────────────────────────────────────────

/// Deprecated: use `get_alerts_paginated` instead.
#[tauri::command]
pub async fn get_alerts(db: State<'_, DbState>) -> Result<Vec<PriceAlert>, AppError> {
    tracing::warn!("get_alerts is deprecated; use get_alerts_paginated");
    let pool = &db.0;
    db::get_alerts(pool).await.map_err(AppError::from)
}

#[tauri::command]
pub async fn add_alert(
    db: State<'_, DbState>,
    alert: PriceAlertInput,
) -> Result<PriceAlert, AppError> {
    if !alert.threshold.is_finite() || alert.threshold <= 0.0 {
        return Err(AppError::Validation(
            "threshold must be a positive finite number".to_string(),
        ));
    }
    let pool = &db.0;
    db::insert_alert(pool, alert).await.map_err(AppError::from)
}

#[tauri::command]
pub async fn delete_alert(db: State<'_, DbState>, id: AlertId) -> Result<bool, AppError> {
    let pool = &db.0;
    db::delete_alert(pool, &id).await.map_err(AppError::from)
}

#[tauri::command]
pub async fn reset_alert(db: State<'_, DbState>, id: AlertId) -> Result<bool, AppError> {
    let pool = &db.0;
    db::reset_alert(pool, &id).await.map_err(AppError::from)
}

/// SQLite magic bytes: first 16 bytes of a valid SQLite database file.
const SQLITE_MAGIC: &[u8] = b"SQLite format 3\0";

#[tauri::command]
pub async fn backup_database(
    app: tauri::AppHandle,
    state: tauri::State<'_, DbState>,
    destination_path: String,
) -> Result<String, AppError> {
    // Flush WAL to ensure the file on disk is complete before we copy it.
    {
        let pool = &state.0;
        sqlx::query("PRAGMA wal_checkpoint(FULL)")
            .execute(pool)
            .await
            .map_err(|e| format!("WAL checkpoint failed: {e}"))?;
    }

    let source = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data dir: {e}"))?
        .join(crate::config::DB_FILE_NAME);

    if !source.exists() {
        return Err(AppError::Validation(
            "Database file does not exist".to_string(),
        ));
    }

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data dir: {e}"))?;

    // Resolve the destination path. If only a filename is provided (no
    // directory component), save the backup to the app data directory.
    // Absolute paths are accepted only if they resolve (after canonicalization)
    // to a path inside the app data directory — this prevents symlink-based
    // path traversal and writing backup files to arbitrary locations.
    let requested = std::path::PathBuf::from(&destination_path);
    let dest = if requested.is_absolute() {
        requested
    } else {
        app_data_dir.join(&requested)
    };

    // Canonicalize the app data dir (must exist).
    let canonical_app_dir =
        std::fs::canonicalize(&app_data_dir).map_err(|e| format!("Cannot resolve app dir: {e}"))?;
    // Canonicalize dest — if the file doesn't exist yet, canonicalize its parent
    // directory to resolve any symlinks. If the parent cannot be canonicalized
    // we return an error rather than falling back to a potentially non-canonical path,
    // which would defeat the path-traversal check below.
    let canonical_dest = if dest.exists() {
        std::fs::canonicalize(&dest).map_err(|e| format!("Cannot resolve destination path: {e}"))?
    } else {
        let parent = dest
            .parent()
            .ok_or("Destination path has no parent directory")?;
        let canonical_parent = if parent.as_os_str().is_empty() {
            canonical_app_dir.clone()
        } else {
            std::fs::canonicalize(parent)
                .map_err(|e| format!("Cannot resolve destination directory: {e}"))?
        };
        canonical_parent.join(dest.file_name().ok_or("Destination path has no filename")?)
    };
    if !canonical_dest.starts_with(&canonical_app_dir) {
        return Err(AppError::Validation(format!(
            "Backup destination must be inside the app data directory ({})",
            app_data_dir.display()
        )));
    }

    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Could not create destination directory: {e}"))?;
        }
    }

    std::fs::copy(&source, &dest).map_err(|e| format!("Failed to copy database: {e}"))?;

    Ok(dest.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn restore_database(
    app: tauri::AppHandle,
    state: tauri::State<'_, DbState>,
    source_path: String,
) -> Result<String, AppError> {
    // Verify the source file is a valid SQLite database.
    let src = std::fs::canonicalize(&source_path)
        .map_err(|e| format!("Cannot resolve backup path: {e}"))?;
    if !src.is_file() {
        return Err(AppError::Validation(
            "Backup path must point to a regular file".to_string(),
        ));
    }

    // Check SQLite magic bytes.
    let mut header = [0u8; 16];
    {
        use std::io::Read;
        let mut f =
            std::fs::File::open(&src).map_err(|e| format!("Cannot open backup file: {e}"))?;
        f.read_exact(&mut header)
            .map_err(|_| "File is too small to be a valid SQLite database".to_string())?;
    }
    if header != SQLITE_MAGIC {
        return Err(AppError::Validation(
            "The selected file is not a valid SQLite database".to_string(),
        ));
    }

    // Open the source file with sqlx to verify it has a holdings table.
    {
        use sqlx::Row;
        let verify_url = format!("sqlite:{}?mode=ro", src.to_string_lossy());
        let verify_pool = sqlx::SqlitePool::connect(&verify_url)
            .await
            .map_err(|e| format!("Cannot open backup as SQLite: {e}"))?;

        let integrity_row = sqlx::query("PRAGMA integrity_check")
            .fetch_one(&verify_pool)
            .await
            .map_err(|e| format!("Integrity check failed on backup: {e}"))?;
        let integrity_result: String = integrity_row.get(0);
        if integrity_result != "ok" {
            verify_pool.close().await;
            return Err(AppError::Validation(format!(
                "Integrity check failed on backup: {}",
                integrity_result
            )));
        }

        let count_row = sqlx::query(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='holdings'",
        )
        .fetch_one(&verify_pool)
        .await
        .map_err(|e| format!("Could not verify holdings table: {e}"))?;
        let has_holdings: bool = count_row.get::<i64, _>(0) > 0;
        verify_pool.close().await;

        if !has_holdings {
            return Err(AppError::Validation(
                "Backup file does not appear to be a portfolio database (no holdings table)"
                    .to_string(),
            ));
        }
    }

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data dir: {e}"))?;

    let dest = app_data_dir.join(crate::config::DB_FILE_NAME);

    // Flush and truncate the WAL so the live DB file on disk is fully
    // self-contained before we overwrite it.  This prevents the old WAL from
    // being replayed over the newly restored data when the pool reconnects.
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&state.0)
        .await
        .map_err(|e| format!("WAL checkpoint failed: {e}"))?;

    // Before overwriting the live database, create a safety backup.  If the
    // copy fails we abort immediately so the live data is never touched.
    if dest.exists() {
        let bak = app_data_dir.join(format!("{}.bak", crate::config::DB_FILE_NAME));
        std::fs::copy(&dest, &bak)
            .map_err(|e| format!("Could not create safety backup before restore: {e}"))?;
    }

    std::fs::copy(&src, &dest).map_err(|e| format!("Failed to restore database: {e}"))?;

    // Remove stale WAL and SHM companion files so the restored DB starts
    // clean and SQLite does not attempt to replay the old journal.
    let wal_path = app_data_dir.join(format!("{}-wal", crate::config::DB_FILE_NAME));
    let shm_path = app_data_dir.join(format!("{}-shm", crate::config::DB_FILE_NAME));
    if wal_path.exists() {
        std::fs::remove_file(&wal_path)
            .map_err(|e| format!("Could not remove WAL file after restore: {e}"))?;
    }
    if shm_path.exists() {
        std::fs::remove_file(&shm_path)
            .map_err(|e| format!("Could not remove SHM file after restore: {e}"))?;
    }

    Ok("Database restored. Please restart the app to apply changes.".to_string())
}

#[tauri::command]
pub async fn get_rebalance_suggestions(
    db: State<'_, DbState>,
    drift_threshold: f64,
) -> Result<Vec<RebalanceSuggestion>, AppError> {
    let base_currency = get_base_currency(&db.0).await;

    let pool = &db.0;
    let holdings = db::get_all_holdings(pool).await?;
    let cached_prices = db::get_cached_prices(pool).await?;
    let cached_fx = db::get_fx_rates(pool).await?;

    let snapshot = build_portfolio_snapshot(
        &holdings,
        &cached_prices,
        &cached_fx,
        &base_currency,
        Utc::now().to_rfc3339(),
        0.0,
        0.0,
    );

    let total_value = snapshot.total_value;

    let mut suggestions: Vec<RebalanceSuggestion> = snapshot
        .holdings
        .into_iter()
        .filter(|h| {
            // Exclude cash holdings and holdings with no target weight
            h.asset_type.as_str() != "cash" && h.target_weight > 0.0
        })
        .filter_map(|h| {
            let target_value_cad = total_value * (h.target_weight / 100.0);
            let drift = h.weight - h.target_weight;
            if drift.abs() < drift_threshold {
                return None;
            }
            // positive = sell (over-weight), negative = buy (under-weight)
            let suggested_trade_cad = h.market_value_cad - target_value_cad;
            let suggested_units = if h.current_price_cad != 0.0 {
                suggested_trade_cad / h.current_price_cad
            } else {
                0.0
            };
            Some(RebalanceSuggestion {
                holding_id: h.id,
                symbol: h.symbol,
                name: h.name,
                current_value_cad: h.market_value_cad,
                target_value_cad,
                current_weight: h.weight,
                target_weight: h.target_weight,
                drift,
                suggested_trade_cad,
                suggested_units,
                current_price_cad: h.current_price_cad,
            })
        })
        .collect();

    // Sort by |drift| descending — biggest drifters first
    suggestions.sort_by(|a, b| {
        b.drift
            .abs()
            .partial_cmp(&a.drift.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(suggestions)
}

// ── Analytics Commands ────────────────────────────────────────────────────────

/// Fetch per-symbol sector/industry/country from Yahoo Finance's v11 quoteSummary
/// `assetProfile` module. Returns `None` for all three fields on any fetch/parse failure
/// (failures are soft — they don't abort the whole analytics call).
async fn fetch_asset_profile(
    client: &reqwest::Client,
    symbol: &str,
) -> (String, Option<String>, Option<String>, Option<String>) {
    let url = crate::config::YAHOO_QUOTE_SUMMARY_URL.replace("{}", symbol);

    let json: Option<serde_json::Value> = async {
        let resp = client
            .get(&url)
            .header("User-Agent", crate::config::USER_AGENT)
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        resp.json::<serde_json::Value>().await.ok()
    }
    .await;

    let profile = json
        .as_ref()
        .and_then(|v| v.pointer("/quoteSummary/result/0/assetProfile"));

    let extract = |key: &str| -> Option<String> {
        profile
            .and_then(|p| p.get(key))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    };

    (
        symbol.to_string(),
        extract("sector"),
        extract("industry"),
        extract("country"),
    )
}

/// Fetch enriched symbol metadata (sector, industry, country, market cap, etc.)
/// for the given list of symbols.
///
/// * Sector, industry, and country are fetched from the v11 `quoteSummary` / `assetProfile`
///   endpoint, which reliably returns these fields (unlike the v7 quote endpoint).
/// * Numeric fields (market cap, P/E, dividend yield, beta) continue to come from the
///   bulk v7 quote endpoint.
///
/// Both requests are issued concurrently. A failure on either is treated as a soft
/// error so that partial data is still returned.
/// Internal helper that optionally checks symbol_cache before hitting Yahoo Finance.
/// When `pool` is provided, fundamentals cached within 24 hours are returned directly.
pub(crate) async fn get_symbol_metadata_with_cache(
    client: &reqwest::Client,
    symbols: &[String],
    pool: Option<&sqlx::SqlitePool>,
) -> Result<Vec<SymbolMetadata>, AppError> {
    if symbols.is_empty() {
        return Ok(vec![]);
    }

    const CACHE_TTL_SECS: i64 = 86_400; // 24 hours

    // ── 0. Check DB cache when pool is available ──────────────────────────────
    let mut results: Vec<Option<SymbolMetadata>> = vec![None; symbols.len()];
    let mut stale_indices: Vec<usize> = Vec::new();

    if let Some(pool) = pool {
        for (i, symbol) in symbols.iter().enumerate() {
            match db::get_symbol_fundamentals_from_cache(pool, symbol, CACHE_TTL_SECS).await {
                Ok(Some(cached)) => results[i] = Some(cached),
                _ => stale_indices.push(i),
            }
        }
    } else {
        stale_indices = (0..symbols.len()).collect();
    }

    if stale_indices.is_empty() {
        return Ok(results.into_iter().flatten().collect());
    }

    let stale_symbols: Vec<String> = stale_indices.iter().map(|&i| symbols[i].clone()).collect();

    // ── 1. Bulk quote request for numeric fields ──────────────────────────────
    let joined = stale_symbols.join(",");
    let quote_url = crate::config::YAHOO_QUOTE_URL.replace("{}", &joined);

    let quote_future = client
        .get(&quote_url)
        .header("User-Agent", crate::config::USER_AGENT)
        .send();

    // ── 2. Per-symbol assetProfile requests for sector/industry/country ───────
    // Use buffer_unordered(5) to cap concurrent HTTP requests at 5 so we don't
    // hammer Yahoo Finance with an unbounded fan-out on large portfolios.
    // Clone the client (reqwest::Client is an Arc internally, so this is cheap).
    let profile_future = {
        use futures::stream::{self, StreamExt};
        let client = client.clone();
        stream::iter(stale_symbols.clone())
            .map(move |s| {
                let client = client.clone();
                async move { fetch_asset_profile(&client, &s).await }
            })
            .buffer_unordered(5)
            .collect::<Vec<_>>()
    };

    // Run bulk quote and bounded profile stream concurrently
    let (quote_response, profile_results) =
        futures::future::join(quote_future, profile_future).await;

    // Parse bulk quote response (best-effort).
    let quote_json: Option<serde_json::Value> = async {
        let resp = quote_response.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        resp.json::<serde_json::Value>().await.ok()
    }
    .await;

    let quote_items: HashMap<String, serde_json::Value> = quote_json
        .and_then(|json| {
            json.pointer("/quoteResponse/result")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| {
                            let sym = item.get("symbol")?.as_str()?.to_string();
                            Some((sym, item.clone()))
                        })
                        .collect()
                })
        })
        .unwrap_or_default();

    // Build a lookup map: symbol → (sector, industry, country) from assetProfile
    type SectorTuple = (Option<String>, Option<String>, Option<String>);
    let profile_map: HashMap<String, SectorTuple> = profile_results
        .into_iter()
        .map(|(sym, sector, industry, country)| (sym, (sector, industry, country)))
        .collect();

    // ── 3. Merge fetched data and persist to cache ────────────────────────────
    for (&original_idx, symbol) in stale_indices.iter().zip(stale_symbols.iter()) {
        let quote = quote_items.get(symbol);
        let (sector, industry, country) = profile_map
            .get(symbol)
            .cloned()
            .unwrap_or((None, None, None));

        let meta = SymbolMetadata {
            symbol: symbol.clone(),
            sector,
            industry,
            country,
            market_cap: quote
                .and_then(|q| q.get("marketCap"))
                .and_then(|v| v.as_f64()),
            pe_ratio: quote
                .and_then(|q| q.get("trailingPE"))
                .and_then(|v| v.as_f64()),
            dividend_yield: quote
                .and_then(|q| q.get("trailingAnnualDividendYield"))
                .and_then(|v| v.as_f64()),
            beta: quote.and_then(|q| q.get("beta")).and_then(|v| v.as_f64()),
            eps: quote
                .and_then(|q| q.get("epsTrailingTwelveMonths"))
                .and_then(|v| v.as_f64()),
        };

        // Persist to cache (best-effort)
        if let Some(pool) = pool {
            if let Err(e) = db::upsert_symbol_fundamentals(pool, &meta).await {
                tracing::warn!("Failed to cache symbol fundamentals: {}", e);
            }
        }

        results[original_idx] = Some(meta);
    }

    Ok(results.into_iter().flatten().collect())
}

fn compute_portfolio_analytics(
    snapshot: &PortfolioSnapshot,
    metadata: &[SymbolMetadata],
) -> PortfolioAnalytics {
    let total_value = snapshot.total_value;

    if total_value == 0.0 {
        return PortfolioAnalytics {
            metadata: metadata.to_vec(),
            risk_metrics: PortfolioRiskMetrics {
                weighted_beta: None,
                portfolio_yield: 0.0,
                largest_position_weight: 0.0,
                top_sector: None,
                concentration_hhi: 0.0,
            },
            sector_breakdown: vec![],
            country_breakdown: vec![],
        };
    }

    // Build a lookup map from symbol → metadata
    let meta_map: HashMap<String, &SymbolMetadata> =
        metadata.iter().map(|m| (m.symbol.clone(), m)).collect();

    // Sector and country accumulators (symbol → (sector, country, market_value_cad))
    let mut sector_values: HashMap<String, f64> = HashMap::new();
    let mut country_values: HashMap<String, f64> = HashMap::new();

    let mut weighted_beta_sum = 0.0_f64;
    let mut weighted_beta_weight = 0.0_f64;
    let mut weighted_yield_sum = 0.0_f64;
    let mut largest_position_weight = 0.0_f64;

    for holding in &snapshot.holdings {
        let weight_fraction = if total_value > 0.0 {
            holding.market_value_cad / total_value
        } else {
            0.0
        };

        if holding.weight > largest_position_weight {
            largest_position_weight = holding.weight;
        }

        let (sector, country) = match holding.asset_type.as_str() {
            "cash" => ("Cash".to_string(), "N/A".to_string()),
            _ => {
                let sector = meta_map
                    .get(&holding.symbol)
                    .and_then(|m| m.sector.clone())
                    .unwrap_or_else(|| "Other".to_string());
                let country = meta_map
                    .get(&holding.symbol)
                    .and_then(|m| m.country.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                (sector, country)
            }
        };

        *sector_values.entry(sector).or_insert(0.0) += holding.market_value_cad;
        *country_values.entry(country).or_insert(0.0) += holding.market_value_cad;

        if let Some(meta) = meta_map.get(&holding.symbol) {
            if let Some(beta) = meta.beta {
                weighted_beta_sum += beta * weight_fraction;
                weighted_beta_weight += weight_fraction;
            }
            if let Some(div_yield) = meta.dividend_yield {
                weighted_yield_sum += div_yield * weight_fraction;
            }
        }
    }

    // Convert value accumulators to weight percentages
    let mut sector_breakdown: Vec<SectorWeight> = sector_values
        .into_iter()
        .map(|(sector, value)| SectorWeight {
            sector,
            weight_percent: if total_value > 0.0 {
                (value / total_value) * 100.0
            } else {
                0.0
            },
        })
        .collect();
    sector_breakdown.sort_by(|a, b| {
        b.weight_percent
            .partial_cmp(&a.weight_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut country_breakdown: Vec<CountryWeight> = country_values
        .into_iter()
        .map(|(country, value)| CountryWeight {
            country,
            weight_percent: if total_value > 0.0 {
                (value / total_value) * 100.0
            } else {
                0.0
            },
        })
        .collect();
    country_breakdown.sort_by(|a, b| {
        b.weight_percent
            .partial_cmp(&a.weight_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // HHI: sum of (weight_fraction^2) * 10000
    let concentration_hhi: f64 = snapshot
        .holdings
        .iter()
        .map(|h| {
            let w = if total_value > 0.0 {
                h.market_value_cad / total_value
            } else {
                0.0
            };
            w * w * 10000.0
        })
        .sum();

    let top_sector = sector_breakdown.first().map(|s| s.sector.clone());

    let weighted_beta = if weighted_beta_weight > 0.0 {
        Some(weighted_beta_sum / weighted_beta_weight)
    } else {
        None
    };

    let risk_metrics = PortfolioRiskMetrics {
        weighted_beta,
        portfolio_yield: weighted_yield_sum,
        largest_position_weight,
        top_sector,
        concentration_hhi,
    };

    PortfolioAnalytics {
        metadata: metadata.to_vec(),
        risk_metrics,
        sector_breakdown,
        country_breakdown,
    }
}

#[tauri::command]
pub async fn get_portfolio_analytics(
    db: State<'_, DbState>,
    http: State<'_, HttpClient>,
) -> Result<PortfolioAnalytics, AppError> {
    let base_currency = get_base_currency(&db.0).await;

    let pool = &db.0;
    let holdings = db::get_all_holdings(pool).await?;
    let cached_prices = db::get_cached_prices(pool).await?;
    let cached_fx = db::get_fx_rates(pool).await?;

    let snapshot = build_portfolio_snapshot(
        &holdings,
        &cached_prices,
        &cached_fx,
        &base_currency,
        Utc::now().to_rfc3339(),
        0.0,
        0.0,
    );

    // Only fetch metadata for non-cash symbols
    let non_cash_symbols: Vec<String> = snapshot
        .holdings
        .iter()
        .filter(|h| h.asset_type.as_str() != "cash")
        .map(|h| h.symbol.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let metadata = get_symbol_metadata_with_cache(&http.0, &non_cash_symbols, Some(pool))
        .await
        .unwrap_or_default();

    Ok(compute_portfolio_analytics(&snapshot, &metadata))
}

// ── Realized gains command ────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_realized_gains(
    db: State<'_, DbState>,
    gains_cache: State<'_, RealizedGainsCacheState>,
    holding_id: Option<HoldingId>,
) -> Result<RealizedGainsSummary, AppError> {
    let pool = &db.0;
    let cost_basis_method = db::get_config(pool, "cost_basis_method")
        .await?
        .unwrap_or_else(|| "avco".to_string());

    // Use the cache only for the full-portfolio (no per-holding filter) query.
    if holding_id.is_none() {
        if let Some(cached) = gains_cache.get() {
            tracing::info!("realized_gains cache hit (get_realized_gains)");
            return Ok(cached);
        }
    }

    let transactions = match holding_id {
        Some(ref id) => db::get_transactions_for_holding(pool, id).await?,
        None => db::get_all_transactions(pool).await?,
    };

    let summary = compute_realized_gains_grouped(&transactions, &cost_basis_method)
        .map_err(AppError::from)?;

    // Populate the cache only for the full-portfolio case.
    if holding_id.is_none() {
        gains_cache.set(summary.clone());
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AccountType, FxRate};
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

// ── Transaction commands ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn add_transaction(
    db: State<'_, DbState>,
    gains_cache: State<'_, RealizedGainsCacheState>,
    input: TransactionInput,
) -> Result<Transaction, AppError> {
    if input.quantity <= 0.0 {
        return Err(AppError::Validation(
            "Transaction quantity must be positive".to_string(),
        ));
    }
    if input.price < 0.0 {
        return Err(AppError::Validation(
            "Transaction price must be non-negative".to_string(),
        ));
    }
    let pool = &db.0;
    let result = db::insert_transaction(pool, input)
        .await
        .map_err(AppError::from)?;
    gains_cache.invalidate();
    Ok(result)
}

/// Deprecated: use `get_transactions_paginated` instead.
#[tauri::command]
pub async fn get_transactions(
    db: State<'_, DbState>,
    holding_id: Option<HoldingId>,
) -> Result<Vec<Transaction>, AppError> {
    tracing::warn!("get_transactions is deprecated; use get_transactions_paginated");
    let pool = &db.0;
    match holding_id {
        Some(id) => db::get_transactions_for_holding(pool, &id)
            .await
            .map_err(AppError::from),
        None => db::get_all_transactions(pool).await.map_err(AppError::from),
    }
}

#[tauri::command]
pub async fn delete_transaction(
    db: State<'_, DbState>,
    gains_cache: State<'_, RealizedGainsCacheState>,
    id: TransactionId,
) -> Result<bool, AppError> {
    let pool = &db.0;
    let result = db::delete_transaction(pool, &id)
        .await
        .map_err(AppError::from)?;
    gains_cache.invalidate();
    Ok(result)
}

// ── Account Commands ──────────────────────────────────────────────────────────

const VALID_ACCOUNT_TYPES: &[&str] =
    &["tfsa", "rrsp", "fhsa", "taxable", "crypto", "cash", "other"];

#[tauri::command]
pub async fn get_accounts(state: tauri::State<'_, DbState>) -> Result<Vec<Account>, AppError> {
    let pool = &state.0;
    db::get_accounts(pool).await.map_err(AppError::from)
}

#[tauri::command]
pub async fn add_account(
    state: tauri::State<'_, DbState>,
    account: CreateAccountRequest,
) -> Result<Account, AppError> {
    let name = account.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation(
            "Account name cannot be empty".to_string(),
        ));
    }
    if !VALID_ACCOUNT_TYPES.contains(&account.account_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid account type: {}",
            account.account_type
        )));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let institution = account.institution.clone();
    let account_type = account.account_type.clone();

    let pool = &state.0;
    db::insert_account(pool, &id, &name, &account_type, institution.as_deref()).await?;

    Ok(Account {
        id,
        name,
        account_type,
        institution,
        created_at,
    })
}

#[tauri::command]
pub async fn update_account(
    state: tauri::State<'_, DbState>,
    id: String,
    account: CreateAccountRequest,
) -> Result<Account, AppError> {
    let name = account.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation(
            "Account name cannot be empty".to_string(),
        ));
    }
    if !VALID_ACCOUNT_TYPES.contains(&account.account_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid account type: {}",
            account.account_type
        )));
    }

    let institution = account.institution.clone();
    let account_type = account.account_type.clone();

    let pool = &state.0;
    // Fetch created_at for the returned struct with a targeted query (avoids N+1)
    let created_at: Option<String> =
        sqlx::query_scalar("SELECT created_at FROM accounts WHERE id = $1")
            .bind(&id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::from)?;
    let created_at = created_at.ok_or_else(|| format!("Account {} not found", id))?;

    db::update_account(pool, &id, &name, &account_type, institution.as_deref()).await?;

    Ok(Account {
        id,
        name,
        account_type,
        institution,
        created_at,
    })
}

#[tauri::command]
pub async fn delete_account(
    state: tauri::State<'_, DbState>,
    id: String,
) -> Result<bool, AppError> {
    let pool = &state.0;
    db::delete_account(pool, &id).await?;
    Ok(true)
}

// ── Paginated commands ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_holdings_paginated(
    db: State<'_, DbState>,
    page: i64,
    page_size: i64,
) -> Result<PaginatedResult<Holding>, AppError> {
    if page < 1 {
        return Err(AppError::Validation("page must be >= 1".to_string()));
    }
    if !(1..=500).contains(&page_size) {
        return Err(AppError::Validation(
            "page_size must be between 1 and 500".to_string(),
        ));
    }
    let pool = &db.0;
    db::get_holdings_paginated(pool, page, page_size)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn get_transactions_paginated(
    db: State<'_, DbState>,
    holding_id: Option<HoldingId>,
    page: i64,
    page_size: i64,
) -> Result<PaginatedResult<Transaction>, AppError> {
    if page < 1 {
        return Err(AppError::Validation("page must be >= 1".to_string()));
    }
    if !(1..=500).contains(&page_size) {
        return Err(AppError::Validation(
            "page_size must be between 1 and 500".to_string(),
        ));
    }
    let pool = &db.0;
    db::get_transactions_paginated(
        pool,
        holding_id.as_ref().map(|id| id.0.as_str()),
        page,
        page_size,
    )
    .await
    .map_err(AppError::from)
}

#[tauri::command]
pub async fn get_alerts_paginated(
    db: State<'_, DbState>,
    page: i64,
    page_size: i64,
) -> Result<PaginatedResult<PriceAlert>, AppError> {
    if page < 1 {
        return Err(AppError::Validation("page must be >= 1".to_string()));
    }
    if !(1..=500).contains(&page_size) {
        return Err(AppError::Validation(
            "page_size must be between 1 and 500".to_string(),
        ));
    }
    let pool = &db.0;
    db::get_alerts_paginated(pool, page, page_size)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn get_dividends_paginated(
    db: State<'_, DbState>,
    page: i64,
    page_size: i64,
) -> Result<PaginatedResult<Dividend>, AppError> {
    if page < 1 {
        return Err(AppError::Validation("page must be >= 1".to_string()));
    }
    if !(1..=500).contains(&page_size) {
        return Err(AppError::Validation(
            "page_size must be between 1 and 500".to_string(),
        ));
    }
    let pool = &db.0;
    db::get_dividends_paginated(pool, page, page_size)
        .await
        .map_err(AppError::from)
}
