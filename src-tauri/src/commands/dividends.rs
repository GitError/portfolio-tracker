use tauri::State;

use crate::db;
use crate::error::AppError;
use crate::types::{Dividend, DividendInput, PaginatedResult};

use super::DbState;

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
