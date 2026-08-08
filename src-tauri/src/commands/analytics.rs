use std::collections::HashMap;

use chrono::Utc;
use tauri::State;

use crate::analytics::compute_realized_gains_grouped;
use crate::db;
use crate::error::AppError;
use crate::portfolio::build_portfolio_snapshot;
use crate::types::{
    AssetType, CountryWeight, HoldingId, HoldingWithPrice, PortfolioAnalytics,
    PortfolioRiskMetrics, PortfolioSnapshot, RealizedGainsSummary, RebalanceSuggestion,
    SectorWeight, SymbolMetadata, Transaction,
};

use super::{
    get_base_currency, normalize_cost_basis_method, DbState, HttpClient, RealizedGainsCacheState,
};

/// Fetch per-symbol sector/industry/country from Yahoo Finance's v11 quoteSummary
/// `assetProfile` module. Returns `None` for all three fields on any fetch/parse failure
/// (failures are soft — they don't abort the whole analytics call).
async fn fetch_asset_profile(
    client: &reqwest::Client,
    symbol: &str,
) -> (String, Option<String>, Option<String>, Option<String>) {
    // Symbols originate from stored holdings, which aren't validated on insert,
    // so guard against an unvalidated value reaching the URL (see #670).
    if let Err(e) = crate::price::validate_symbol(symbol) {
        tracing::warn!("Skipping asset profile fetch for invalid symbol: {}", e);
        return (symbol.to_string(), None, None, None);
    }
    let encoded_symbol = urlencoding::encode(symbol);
    let url = crate::config::YAHOO_QUOTE_SUMMARY_URL.replace("{}", &encoded_symbol);

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
) -> Vec<SymbolMetadata> {
    if symbols.is_empty() {
        return vec![];
    }

    const CACHE_TTL_SECS: i64 = 86_400; // 24 hours

    // ── 0. Check DB cache when pool is available ──────────────────────────────
    let mut results: Vec<Option<SymbolMetadata>> = vec![None; symbols.len()];
    let mut stale_indices: Vec<usize> = Vec::new();

    if let Some(pool) = pool {
        // Single batch query instead of one query per symbol (fixes N+1, #738).
        let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
        let cached_batch =
            db::get_symbol_fundamentals_from_cache_batch(pool, &symbol_refs, CACHE_TTL_SECS)
                .await
                .unwrap_or_default();

        // Build an O(1) lookup map (owned-key so no lifetime issues).
        let cached_map: std::collections::HashMap<String, SymbolMetadata> = cached_batch
            .into_iter()
            .map(|m| (m.symbol.clone(), m))
            .collect();

        for (i, symbol) in symbols.iter().enumerate() {
            if let Some(meta) = cached_map.get(symbol) {
                results[i] = Some(meta.clone());
            } else {
                stale_indices.push(i);
            }
        }
    } else {
        stale_indices = (0..symbols.len()).collect();
    }

    if stale_indices.is_empty() {
        return results.into_iter().flatten().collect();
    }

    let stale_symbols: Vec<String> = stale_indices.iter().map(|&i| symbols[i].clone()).collect();

    // ── 1. Bulk quote request for numeric fields ──────────────────────────────
    // Validate and encode each symbol individually before joining so a single
    // malformed symbol can't corrupt the query string or smuggle in extra
    // characters (see #670); each encoded symbol is comma-safe since the
    // allowed charset contains no commas.
    let joined = stale_symbols
        .iter()
        .filter(|s| match crate::price::validate_symbol(s) {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!("Skipping invalid symbol in bulk quote request: {}", e);
                false
            }
        })
        .map(|s| urlencoding::encode(s).into_owned())
        .collect::<Vec<_>>()
        .join(",");
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

        // Persist to cache (best-effort). Pull name/asset_type/exchange/currency
        // from the same bulk quote response so a brand-new symbol_cache row gets
        // real metadata instead of a hardcoded 'stock'/blank placeholder (#610).
        let name = quote
            .and_then(|q| q.get("longName").or_else(|| q.get("shortName")))
            .and_then(|v| v.as_str());
        let exchange = quote
            .and_then(|q| q.get("fullExchangeName").or_else(|| q.get("exchange")))
            .and_then(|v| v.as_str());
        let currency = quote
            .and_then(|q| q.get("currency"))
            .and_then(|v| v.as_str());
        // Mirrors the quoteType → AssetType mapping in search.rs's symbol search.
        let asset_type = quote
            .and_then(|q| q.get("quoteType"))
            .and_then(|v| v.as_str())
            .map(|quote_type| match quote_type {
                "ETF" | "MUTUALFUND" => AssetType::Etf,
                "CRYPTOCURRENCY" => AssetType::Crypto,
                _ => AssetType::Stock,
            });

        if let Some(pool) = pool {
            if let Err(e) =
                db::upsert_symbol_fundamentals(pool, &meta, name, asset_type, exchange, currency)
                    .await
            {
                tracing::warn!("Failed to cache symbol fundamentals: {}", e);
            }
        }

        results[original_idx] = Some(meta);
    }

    results.into_iter().flatten().collect()
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

    let metadata = get_symbol_metadata_with_cache(&http.0, &non_cash_symbols, Some(pool)).await;

    Ok(compute_portfolio_analytics(&snapshot, &metadata))
}

#[tauri::command]
pub async fn get_realized_gains(
    db: State<'_, DbState>,
    gains_cache: State<'_, RealizedGainsCacheState>,
    holding_id: Option<HoldingId>,
) -> Result<RealizedGainsSummary, AppError> {
    let pool = &db.0;
    let cost_basis_method =
        normalize_cost_basis_method(db::get_config(pool, "cost_basis_method").await?);

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

    let base_currency = get_base_currency(pool).await;
    let cached_fx = db::get_fx_rates(pool).await?;
    let holding_currencies = db::get_all_holding_currencies(pool).await?;

    let summary = compute_realized_gains_grouped(
        &transactions,
        &cost_basis_method,
        &holding_currencies,
        &base_currency,
        &cached_fx,
    )
    .map_err(AppError::from)?;

    // Populate the cache only for the full-portfolio case.
    if holding_id.is_none() {
        gains_cache.set(summary.clone());
    }

    Ok(summary)
}

/// A negative, NaN/infinite, or >100 `drift_threshold` would silently disable
/// or misapply the drift filter in `compute_rebalance_suggestions` — reject it
/// at the command boundary instead.
fn validate_drift_threshold(drift_threshold: f64) -> Result<(), AppError> {
    if !drift_threshold.is_finite() || !(0.0..=100.0).contains(&drift_threshold) {
        return Err(AppError::Validation(
            "drift_threshold must be a finite number between 0 and 100".to_string(),
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn get_rebalance_suggestions(
    db: State<'_, DbState>,
    drift_threshold: f64,
) -> Result<Vec<RebalanceSuggestion>, AppError> {
    validate_drift_threshold(drift_threshold)?;
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

    Ok(compute_rebalance_suggestions(
        snapshot.holdings,
        snapshot.total_value,
        drift_threshold,
    ))
}

/// Pure computation behind `get_rebalance_suggestions`, split out for unit testing.
///
/// `target_weight` is `None` when the user has never set a target for a holding,
/// and `Some(w)` when they have (including `Some(0.0)` for an explicit 0% target).
/// A holding with no target set is excluded from suggestions entirely — there is
/// nothing to rebalance towards. A holding *explicitly* targeted at 0% is always
/// a full-drift "sell everything" candidate: its drift equals its entire current
/// weight, so it must bypass `drift_threshold` instead of being skipped like a
/// normal below-threshold drift.
fn compute_rebalance_suggestions(
    holdings: Vec<HoldingWithPrice>,
    total_value: f64,
    drift_threshold: f64,
) -> Vec<RebalanceSuggestion> {
    let mut suggestions: Vec<RebalanceSuggestion> = holdings
        .into_iter()
        .filter(|h| h.asset_type.as_str() != "cash")
        .filter_map(|h| {
            // No target set at all — nothing to rebalance towards.
            let target_weight = h.holding.target_weight?;
            let is_explicit_zero_target = target_weight == 0.0;
            if is_explicit_zero_target && h.weight == 0.0 {
                // Explicitly targeted at 0% but never held — no suggestion to make.
                return None;
            }
            let target_value_cad = total_value * (target_weight / 100.0);
            let drift = h.weight - target_weight;
            if !is_explicit_zero_target && drift.abs() < drift_threshold {
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
                holding_id: h.holding.id,
                symbol: h.holding.symbol,
                name: h.holding.name,
                current_value_cad: h.market_value_cad,
                target_value_cad,
                current_weight: h.weight,
                target_weight,
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

    suggestions
}

#[cfg(test)]
mod validate_drift_threshold_tests {
    use super::validate_drift_threshold;

    #[test]
    fn rejects_negative_threshold() {
        assert!(validate_drift_threshold(-0.01).is_err());
    }

    #[test]
    fn rejects_threshold_above_100() {
        assert!(validate_drift_threshold(100.01).is_err());
    }

    #[test]
    fn rejects_nan_and_infinite_threshold() {
        assert!(validate_drift_threshold(f64::NAN).is_err());
        assert!(validate_drift_threshold(f64::INFINITY).is_err());
        assert!(validate_drift_threshold(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn accepts_boundary_and_typical_values() {
        assert!(validate_drift_threshold(0.0).is_ok());
        assert!(validate_drift_threshold(100.0).is_ok());
        assert!(validate_drift_threshold(5.0).is_ok());
    }
}

#[cfg(test)]
mod compute_rebalance_suggestions_tests {
    use super::compute_rebalance_suggestions;
    use crate::types::{AccountType, AssetType, Holding, HoldingId, HoldingWithPrice};

    fn holding_with_price(
        symbol: &str,
        asset_type: AssetType,
        target_weight: Option<f64>,
        weight: f64,
        market_value_cad: f64,
        current_price_cad: f64,
    ) -> HoldingWithPrice {
        HoldingWithPrice {
            holding: Holding {
                id: HoldingId(symbol.to_string()),
                symbol: symbol.to_string(),
                name: symbol.to_string(),
                asset_type,
                account: AccountType::Taxable,
                account_id: None,
                account_name: None,
                quantity: 10.0,
                cost_basis: 10.0,
                currency: "CAD".to_string(),
                exchange: String::new(),
                target_weight,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
                indicated_annual_dividend: None,
                indicated_annual_dividend_currency: None,
                dividend_frequency: None,
                maturity_date: None,
            },
            current_price: current_price_cad,
            current_price_cad,
            market_value_cad,
            cost_value_cad: market_value_cad,
            gain_loss: 0.0,
            gain_loss_percent: 0.0,
            weight,
            target_value: 0.0,
            target_delta_value: 0.0,
            target_delta_percent: 0.0,
            daily_change_percent: 0.0,
            fx_stale: false,
            price_is_stale: false,
        }
    }

    #[test]
    fn no_target_set_is_excluded_even_when_held_and_above_threshold() {
        // Regression guard for #608: held at 15% of the portfolio but the user
        // never set a target (None) — must NOT be treated as a 0% target and
        // must NOT produce a "sell everything" suggestion, even with a
        // drift_threshold far below its current weight.
        let holdings = vec![holding_with_price(
            "XYZ",
            AssetType::Stock,
            None,
            15.0,
            1500.0,
            50.0,
        )];

        let suggestions = compute_rebalance_suggestions(holdings, 10_000.0, 1.0);

        assert!(
            suggestions.is_empty(),
            "holding with no target set must be excluded entirely, got: {:?}",
            suggestions
        );
    }

    #[test]
    fn no_target_set_and_zero_weight_produces_no_suggestion() {
        let holdings = vec![holding_with_price(
            "NONE",
            AssetType::Stock,
            None,
            0.0,
            0.0,
            50.0,
        )];

        let suggestions = compute_rebalance_suggestions(holdings, 10_000.0, 1.0);

        assert!(suggestions.is_empty());
    }

    #[test]
    fn explicit_zero_target_always_suggests_selling_everything() {
        // Held at 15% of the portfolio but explicitly targeted at 0% — should
        // always appear, even with a drift_threshold far above its current weight.
        let holdings = vec![holding_with_price(
            "XYZ",
            AssetType::Stock,
            Some(0.0),
            15.0,
            1500.0,
            50.0,
        )];

        let suggestions = compute_rebalance_suggestions(holdings, 10_000.0, 50.0);

        assert_eq!(suggestions.len(), 1);
        let s = &suggestions[0];
        assert_eq!(s.symbol, "XYZ");
        assert_eq!(s.target_weight, 0.0);
        assert_eq!(s.target_value_cad, 0.0);
        // Drift equals the full current weight — a "sell everything" signal.
        assert_eq!(s.drift, 15.0);
        assert_eq!(s.suggested_trade_cad, 1500.0);
    }

    #[test]
    fn explicit_zero_target_and_zero_weight_produces_no_suggestion() {
        // Explicitly targeted at 0% but never held — nothing to suggest.
        let holdings = vec![holding_with_price(
            "NONE",
            AssetType::Stock,
            Some(0.0),
            0.0,
            0.0,
            50.0,
        )];

        let suggestions = compute_rebalance_suggestions(holdings, 10_000.0, 1.0);

        assert!(suggestions.is_empty());
    }

    #[test]
    fn cash_holdings_are_always_excluded() {
        let holdings = vec![holding_with_price(
            "CASH",
            AssetType::Cash,
            Some(0.0),
            15.0,
            1500.0,
            1.0,
        )];

        let suggestions = compute_rebalance_suggestions(holdings, 10_000.0, 1.0);

        assert!(suggestions.is_empty());
    }

    #[test]
    fn positive_target_below_drift_threshold_is_still_skipped() {
        // Regression guard: non-zero targets must keep respecting drift_threshold.
        let holdings = vec![holding_with_price(
            "ABC",
            AssetType::Stock,
            Some(20.0),
            22.0,
            2200.0,
            50.0,
        )];

        let suggestions = compute_rebalance_suggestions(holdings, 10_000.0, 5.0);

        assert!(suggestions.is_empty());
    }

    #[test]
    fn buy_suggestion_still_fires_for_unheld_target() {
        // Regression guard: a holding not yet owned (weight 0) but with a real
        // target must still generate a buy suggestion.
        let holdings = vec![holding_with_price(
            "NEW",
            AssetType::Stock,
            Some(10.0),
            0.0,
            0.0,
            50.0,
        )];

        let suggestions = compute_rebalance_suggestions(holdings, 10_000.0, 1.0);

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].drift, -10.0);
        assert_eq!(suggestions[0].suggested_trade_cad, -1000.0);
    }
}
