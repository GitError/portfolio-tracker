use std::collections::HashMap;

use chrono::Utc;
use tauri::State;

use crate::db;
use crate::error::AppError;
use crate::fx::fetch_all_fx_rates;
use crate::portfolio::build_portfolio_snapshot;
use crate::price::{fetch_all_prices, fetch_price, FetchAllPricesResult};
use crate::search::search_symbols_yahoo;
use crate::types::{PerformancePoint, PriceData, RefreshResult, SymbolResult};

use super::{get_base_currency, DbState, HttpClient, SearchCacheState};

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
