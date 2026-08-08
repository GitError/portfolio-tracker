use chrono::Utc;
use tauri::State;

use crate::analytics::compute_realized_gains_grouped;
use crate::db;
use crate::error::AppError;
use crate::portfolio::build_portfolio_snapshot;
use crate::types::{Holding, HoldingId, HoldingInput, PortfolioSnapshot};

use super::{
    get_base_currency, normalize_cost_basis_method, validate_holding_dividend_fields,
    validate_holding_fields, validate_id, validate_pagination, validate_target_weight, DbState,
    HttpClient, RealizedGainsCacheState,
};

#[tauri::command]
pub async fn get_portfolio(
    db: State<'_, DbState>,
    _client: State<'_, HttpClient>,
    gains_cache: State<'_, RealizedGainsCacheState>,
) -> Result<PortfolioSnapshot, AppError> {
    get_portfolio_impl(&db.0, &gains_cache).await
}

pub(crate) async fn get_portfolio_impl(
    pool: &sqlx::SqlitePool,
    gains_cache: &RealizedGainsCacheState,
) -> Result<PortfolioSnapshot, AppError> {
    let base_currency = get_base_currency(pool).await;

    let holdings = db::get_all_holdings(pool).await?;

    let cached_prices = db::get_cached_prices(pool).await?;
    let cached_fx = db::get_fx_rates(pool).await?;

    let cost_basis_method_opt = db::get_config(pool, "cost_basis_method").await?;
    // If the user has never explicitly chosen a method, flag the snapshot so the frontend
    // can prompt for an explicit selection before displaying realized gains.
    let requires_cost_basis_selection = cost_basis_method_opt.is_none();
    let cost_basis_method = normalize_cost_basis_method(cost_basis_method_opt);

    let realized_gains = {
        let summary = if let Some(cached) = gains_cache.get() {
            tracing::info!("realized_gains cache hit");
            cached
        } else {
            let transactions = db::get_all_transactions(pool).await?;
            let holding_currencies = db::get_all_holding_currencies(pool).await?;
            match compute_realized_gains_grouped(
                &transactions,
                &cost_basis_method,
                &holding_currencies,
                &base_currency,
                &cached_fx,
            ) {
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

    let annual_dividend_income =
        db::get_annual_dividend_income(pool, &base_currency, &cached_fx).await?;

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
    validate_pagination(page, page_size)?;
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
    add_holding_impl(&db.0, holding).await
}

async fn add_holding_impl(
    pool: &sqlx::SqlitePool,
    holding: HoldingInput,
) -> Result<Holding, AppError> {
    let currency =
        validate_holding_fields(holding.quantity, holding.cost_basis, &holding.currency)?;
    validate_target_weight(holding.target_weight)?;
    validate_holding_dividend_fields(
        holding.indicated_annual_dividend,
        holding.dividend_frequency.as_deref(),
        holding.maturity_date.as_deref(),
    )?;
    let holding = HoldingInput {
        currency,
        ..holding
    };
    if let Some(target_weight) = holding.target_weight {
        if target_weight > 0.0 {
            let current_sum = db::sum_target_weights(pool, None).await?;
            if portfolio_core::validation::exceeds_target_weight_budget(target_weight, current_sum)
            {
                return Err(AppError::Validation(format!(
                    "Total target weight would exceed 100% (currently {:.1}%). Adjust existing allocations before adding this holding.",
                    current_sum
                )));
            }
        }
    }
    db::insert_holding(pool, holding)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn update_holding(
    db: State<'_, DbState>,
    gains_cache: State<'_, RealizedGainsCacheState>,
    holding: Holding,
) -> Result<Holding, AppError> {
    update_holding_impl(&db.0, &gains_cache, holding).await
}

async fn update_holding_impl(
    pool: &sqlx::SqlitePool,
    gains_cache: &RealizedGainsCacheState,
    holding: Holding,
) -> Result<Holding, AppError> {
    let currency =
        validate_holding_fields(holding.quantity, holding.cost_basis, &holding.currency)?;
    validate_target_weight(holding.target_weight)?;
    validate_holding_dividend_fields(
        holding.indicated_annual_dividend,
        holding.dividend_frequency.as_deref(),
        holding.maturity_date.as_deref(),
    )?;
    let holding = Holding {
        currency,
        ..holding
    };
    if let Some(target_weight) = holding.target_weight {
        if target_weight > 0.0 {
            let current_sum = db::sum_target_weights(pool, Some(holding.id.0.as_str())).await?;
            if portfolio_core::validation::exceeds_target_weight_budget(target_weight, current_sum)
            {
                return Err(AppError::Validation(format!(
                    "Total target weight would exceed 100% (currently {:.1}% across other holdings). Adjust existing allocations before saving.",
                    current_sum
                )));
            }
        }
    }
    // Realized-gains transactions are converted using the holding's *current*
    // currency (see compute_realized_gains_grouped), so a cached summary from
    // before a currency change would silently apply the wrong FX rate (#767).
    let previous_currency = db::get_holding_symbol_and_currency(pool, holding.id.0.as_str())
        .await?
        .map(|(_, currency)| currency);
    let new_currency = holding.currency.clone();
    let updated = db::update_holding(pool, holding)
        .await
        .map_err(AppError::from)?;
    if previous_currency.as_deref() != Some(new_currency.as_str()) {
        gains_cache.invalidate();
    }
    Ok(updated)
}

#[tauri::command]
pub async fn delete_holding(
    db: State<'_, DbState>,
    gains_cache: State<'_, RealizedGainsCacheState>,
    id: HoldingId,
) -> Result<bool, AppError> {
    validate_id("holding ID", &id.0)?;
    delete_holding_impl(&db.0, &gains_cache, &id).await
}

async fn delete_holding_impl(
    pool: &sqlx::SqlitePool,
    gains_cache: &RealizedGainsCacheState,
    id: &HoldingId,
) -> Result<bool, AppError> {
    let deleted = db::delete_holding(pool, id).await.map_err(AppError::from)?;
    if deleted {
        // The holding's transactions remain in realized-gains history after a
        // soft-delete; any cached summary computed before this point may have
        // used a stale holding_currencies map, so force a recompute (#767).
        gains_cache.invalidate();
    }
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::types::{AccountType, AssetType, HoldingInput};

    fn make_input(symbol: &str, currency: &str) -> HoldingInput {
        HoldingInput {
            symbol: symbol.to_string(),
            name: format!("{symbol} Inc."),
            asset_type: AssetType::Stock,
            account: AccountType::Taxable,
            account_id: None,
            quantity: 10.0,
            cost_basis: 150.0,
            currency: currency.to_string(),
            exchange: "TSX".to_string(),
            target_weight: None,
            indicated_annual_dividend: None,
            indicated_annual_dividend_currency: None,
            dividend_frequency: None,
            maturity_date: None,
        }
    }

    #[tokio::test]
    async fn get_portfolio_propagates_annual_dividend_income_error() {
        // Regression guard for #672: get_portfolio previously called
        // `.unwrap_or(0.0)` on get_annual_dividend_income, silently hiding
        // any DB error (e.g. a query failure) behind a fake $0 figure.
        let pool = db::open_test_db().await;
        sqlx::query("DROP TABLE dividends")
            .execute(&pool)
            .await
            .expect("drop dividends table to force a query error");

        let gains_cache = crate::commands::RealizedGainsCacheState::new();
        let result = super::get_portfolio_impl(&pool, &gains_cache).await;
        assert!(
            result.is_err(),
            "get_portfolio must surface the dividend-income query error instead of swallowing it"
        );
    }

    #[tokio::test]
    async fn add_holding_persists_normalized_currency() {
        // Regression guard for #691: validate_holding_fields returns the
        // normalized (trimmed, uppercased) currency, but add_holding
        // discarded it and persisted the raw user input instead.
        let pool = db::open_test_db().await;
        let inserted = super::add_holding_impl(&pool, make_input("RY", "  usd "))
            .await
            .expect("add_holding_impl should succeed");
        assert_eq!(inserted.currency, "USD");

        let fetched = db::get_all_holdings(&pool)
            .await
            .expect("get_all_holdings")
            .into_iter()
            .find(|h| h.id == inserted.id)
            .expect("holding must exist");
        assert_eq!(fetched.currency, "USD");
    }

    #[tokio::test]
    async fn update_holding_persists_normalized_currency() {
        // Same bug as add_holding, for the update path.
        let pool = db::open_test_db().await;
        let inserted = super::add_holding_impl(&pool, make_input("RY", "CAD"))
            .await
            .expect("add_holding_impl should succeed");

        let mut to_update = inserted.clone();
        to_update.currency = "  usd ".to_string();
        let gains_cache = crate::commands::RealizedGainsCacheState::new();
        let updated = super::update_holding_impl(&pool, &gains_cache, to_update)
            .await
            .expect("update_holding_impl should succeed");
        assert_eq!(updated.currency, "USD");

        let fetched = db::get_all_holdings(&pool)
            .await
            .expect("get_all_holdings")
            .into_iter()
            .find(|h| h.id == inserted.id)
            .expect("holding must exist");
        assert_eq!(fetched.currency, "USD");
    }

    #[tokio::test]
    async fn update_holding_invalidates_gains_cache_on_currency_change() {
        // Regression guard for #767: a cached realized-gains summary computed
        // while the holding was CAD must not survive a currency change to USD,
        // since the cached figures were converted with the old currency's FX rate.
        let pool = db::open_test_db().await;
        let inserted = super::add_holding_impl(&pool, make_input("RY", "CAD"))
            .await
            .expect("add_holding_impl should succeed");

        let gains_cache = crate::commands::RealizedGainsCacheState::new();
        gains_cache.set(crate::types::RealizedGainsSummary {
            total_realized_gain: 100.0,
            total_proceeds: 100.0,
            total_cost_basis: 0.0,
            lots: vec![],
        });
        assert!(gains_cache.get().is_some(), "sanity check: cache is warm");

        let mut to_update = inserted.clone();
        to_update.currency = "USD".to_string();
        super::update_holding_impl(&pool, &gains_cache, to_update)
            .await
            .expect("update_holding_impl should succeed");

        assert!(
            gains_cache.get().is_none(),
            "changing a holding's currency must invalidate the cached realized-gains summary"
        );
    }

    #[tokio::test]
    async fn update_holding_does_not_invalidate_gains_cache_when_currency_unchanged() {
        let pool = db::open_test_db().await;
        let inserted = super::add_holding_impl(&pool, make_input("RY", "CAD"))
            .await
            .expect("add_holding_impl should succeed");

        let gains_cache = crate::commands::RealizedGainsCacheState::new();
        gains_cache.set(crate::types::RealizedGainsSummary {
            total_realized_gain: 100.0,
            total_proceeds: 100.0,
            total_cost_basis: 0.0,
            lots: vec![],
        });

        let mut to_update = inserted.clone();
        to_update.quantity = 20.0;
        super::update_holding_impl(&pool, &gains_cache, to_update)
            .await
            .expect("update_holding_impl should succeed");

        assert!(
            gains_cache.get().is_some(),
            "an unrelated field update must not invalidate the realized-gains cache"
        );
    }

    #[tokio::test]
    async fn delete_holding_invalidates_gains_cache() {
        // Regression guard for #767: a soft-deleted holding's historical
        // transactions remain in realized-gains history, so any cached
        // summary computed before the delete must be recomputed.
        let pool = db::open_test_db().await;
        let inserted = super::add_holding_impl(&pool, make_input("RY", "USD"))
            .await
            .expect("add_holding_impl should succeed");

        let gains_cache = crate::commands::RealizedGainsCacheState::new();
        gains_cache.set(crate::types::RealizedGainsSummary {
            total_realized_gain: 100.0,
            total_proceeds: 100.0,
            total_cost_basis: 0.0,
            lots: vec![],
        });
        assert!(gains_cache.get().is_some(), "sanity check: cache is warm");

        let deleted = super::delete_holding_impl(&pool, &gains_cache, &inserted.id)
            .await
            .expect("delete_holding_impl should succeed");
        assert!(deleted);

        assert!(
            gains_cache.get().is_none(),
            "soft-deleting a holding must invalidate the cached realized-gains summary"
        );
    }

    async fn buy_sell_transactions(pool: &sqlx::SqlitePool, holding_id: &crate::types::HoldingId) {
        use crate::types::{TransactionInput, TransactionType};
        db::insert_transaction(
            pool,
            TransactionInput {
                holding_id: holding_id.clone(),
                transaction_type: TransactionType::Buy,
                quantity: 10.0,
                price: 100.0,
                transacted_at: "2024-01-01T00:00:00Z".to_string(),
            },
        )
        .await
        .expect("insert buy transaction");
        db::insert_transaction(
            pool,
            TransactionInput {
                holding_id: holding_id.clone(),
                transaction_type: TransactionType::Sell,
                quantity: 10.0,
                price: 150.0,
                transacted_at: "2024-01-02T00:00:00Z".to_string(),
            },
        )
        .await
        .expect("insert sell transaction");
    }

    #[tokio::test]
    async fn get_portfolio_reflects_new_currency_after_holding_currency_change() {
        // End-to-end regression guard for #767: buy/sell 10 shares for a gain of
        // 500 in the holding's currency. Verify the reported realized gain
        // (in CAD) tracks the FX rate of the holding's *current* currency after
        // an update, not a stale cached figure from the prior currency.
        let pool = db::open_test_db().await;
        let inserted = super::add_holding_impl(&pool, make_input("RY", "USD"))
            .await
            .expect("add_holding_impl should succeed");
        buy_sell_transactions(&pool, &inserted.id).await;

        db::upsert_fx_rate(
            &pool,
            &crate::types::FxRate {
                pair: "USDCAD".to_string(),
                rate: 1.30,
                updated_at: "2024-01-01T00:00:00Z".to_string(),
            },
        )
        .await
        .expect("upsert USDCAD rate");
        db::upsert_fx_rate(
            &pool,
            &crate::types::FxRate {
                pair: "EURCAD".to_string(),
                rate: 1.45,
                updated_at: "2024-01-01T00:00:00Z".to_string(),
            },
        )
        .await
        .expect("upsert EURCAD rate");

        let gains_cache = crate::commands::RealizedGainsCacheState::new();
        let snapshot_before = super::get_portfolio_impl(&pool, &gains_cache)
            .await
            .expect("get_portfolio_impl should succeed");
        // gain = (150 - 100) * 10 = 500 USD -> 500 * 1.30 CAD
        assert!(
            (snapshot_before.realized_gains - 650.0).abs() < 0.001,
            "expected USD-converted realized gain of 650.0, got {}",
            snapshot_before.realized_gains
        );

        let mut to_update = inserted.clone();
        to_update.currency = "EUR".to_string();
        super::update_holding_impl(&pool, &gains_cache, to_update)
            .await
            .expect("update_holding_impl should succeed");

        let snapshot_after = super::get_portfolio_impl(&pool, &gains_cache)
            .await
            .expect("get_portfolio_impl should succeed");
        // gain = (150 - 100) * 10 = 500 EUR -> 500 * 1.45 CAD
        assert!(
            (snapshot_after.realized_gains - 725.0).abs() < 0.001,
            "expected EUR-converted realized gain of 725.0 after currency change, got {}",
            snapshot_after.realized_gains
        );
    }

    #[tokio::test]
    async fn get_portfolio_converts_deleted_holdings_transactions_using_original_currency() {
        // End-to-end regression guard for #767: soft-deleting a non-base-currency
        // holding must not make its historical transactions fall back to a 1:1
        // base-currency conversion. Without the fix, holding_currencies is built
        // only from active holdings, so a deleted USD holding's gain would be
        // reported as 500 (raw) instead of 650 (converted at 1.30 USDCAD).
        let pool = db::open_test_db().await;
        let inserted = super::add_holding_impl(&pool, make_input("RY", "USD"))
            .await
            .expect("add_holding_impl should succeed");
        buy_sell_transactions(&pool, &inserted.id).await;

        db::upsert_fx_rate(
            &pool,
            &crate::types::FxRate {
                pair: "USDCAD".to_string(),
                rate: 1.30,
                updated_at: "2024-01-01T00:00:00Z".to_string(),
            },
        )
        .await
        .expect("upsert USDCAD rate");

        let gains_cache = crate::commands::RealizedGainsCacheState::new();
        let deleted = super::delete_holding_impl(&pool, &gains_cache, &inserted.id)
            .await
            .expect("delete_holding_impl should succeed");
        assert!(deleted);

        let snapshot = super::get_portfolio_impl(&pool, &gains_cache)
            .await
            .expect("get_portfolio_impl should succeed");
        assert!(
            (snapshot.realized_gains - 650.0).abs() < 0.001,
            "expected the deleted USD holding's gain to still be converted at 1.30 USDCAD (650.0), got {}",
            snapshot.realized_gains
        );
    }
}
