use chrono::Utc;
use tauri::State;

use crate::analytics::compute_realized_gains_grouped;
use crate::db;
use crate::error::AppError;
use crate::portfolio::build_portfolio_snapshot;
use crate::types::{Holding, HoldingId, HoldingInput, PortfolioSnapshot};

use super::{get_base_currency, validate_holding_fields, DbState, HttpClient, RealizedGainsCacheState, WEIGHT_EPSILON};

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

#[tauri::command]
pub async fn get_holdings_paginated(
    db: State<'_, DbState>,
    page: i64,
    page_size: i64,
) -> Result<crate::types::PaginatedResult<Holding>, AppError> {
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
