use tauri::State;

use crate::db;
use crate::error::AppError;
use crate::types::{HoldingId, PaginatedResult, Transaction, TransactionId, TransactionInput};

use super::{DbState, RealizedGainsCacheState};

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
