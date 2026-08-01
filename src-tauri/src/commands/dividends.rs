use tauri::State;

use crate::db;
use crate::error::AppError;
use crate::types::{Dividend, DividendId, DividendInput, PaginatedResult};

use super::{validate_dividend_fields, validate_pagination, DbState};

/// Deprecated: use `get_dividends_paginated` instead.
#[tauri::command]
pub async fn get_dividends(db: State<'_, DbState>) -> Result<Vec<Dividend>, AppError> {
    tracing::warn!("get_dividends is deprecated; use get_dividends_paginated");
    let pool = &db.0;
    db::get_dividends(pool).await.map_err(AppError::from)
}

#[tauri::command]
pub async fn get_dividends_paginated(
    db: State<'_, DbState>,
    page: i64,
    page_size: i64,
) -> Result<PaginatedResult<Dividend>, AppError> {
    validate_pagination(page, page_size)?;
    let pool = &db.0;
    db::get_dividends_paginated(pool, page, page_size)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn add_dividend(
    db: State<'_, DbState>,
    dividend: DividendInput,
) -> Result<Dividend, AppError> {
    validate_dividend_fields(
        dividend.amount_per_unit,
        &dividend.ex_date,
        &dividend.pay_date,
    )?;
    let pool = &db.0;
    let (symbol, holding_currency) =
        db::get_holding_symbol_and_currency(pool, dividend.holding_id.0.as_str())
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Holding {} not found", dividend.holding_id.0))
            })?;
    // Validate that the dividend currency matches the holding's currency.
    if holding_currency.to_uppercase() != dividend.currency.to_uppercase() {
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
pub async fn delete_dividend(db: State<'_, DbState>, id: DividendId) -> Result<bool, AppError> {
    let pool = &db.0;
    db::delete_dividend(pool, &id).await.map_err(AppError::from)
}
