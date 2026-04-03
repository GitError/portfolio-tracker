use std::collections::HashMap;

use chrono::Utc;
use tauri::State;

use crate::analytics::compute_realized_gains_grouped;
use crate::db;
use crate::error::AppError;
use crate::portfolio::build_portfolio_snapshot;
use crate::types::{
    CountryWeight, HoldingId, PortfolioAnalytics, PortfolioRiskMetrics,
    PortfolioSnapshot, RealizedGainsSummary, RebalanceSuggestion, SectorWeight, SymbolMetadata,
    Transaction,
};

use super::{get_base_currency, DbState, HttpClient, RealizedGainsCacheState};

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

    let transactions: Vec<Transaction> = match holding_id {
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
