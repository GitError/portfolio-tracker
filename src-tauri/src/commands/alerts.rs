use tauri::State;

use crate::db;
use crate::error::AppError;
use crate::types::{AlertId, PaginatedResult, PriceAlert, PriceAlertInput};

use super::{validate_alert_fields, validate_id, validate_pagination, DbState};

/// Deprecated: use `get_alerts_paginated` instead.
#[tauri::command]
pub async fn get_alerts(db: State<'_, DbState>) -> Result<Vec<PriceAlert>, AppError> {
    tracing::warn!("get_alerts is deprecated; use get_alerts_paginated");
    let pool = &db.0;
    db::get_alerts(pool).await.map_err(AppError::from)
}

#[tauri::command]
pub async fn get_alerts_paginated(
    db: State<'_, DbState>,
    page: i64,
    page_size: i64,
) -> Result<PaginatedResult<PriceAlert>, AppError> {
    validate_pagination(page, page_size)?;
    let pool = &db.0;
    db::get_alerts_paginated(pool, page, page_size)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn add_alert(
    db: State<'_, DbState>,
    alert: PriceAlertInput,
) -> Result<PriceAlert, AppError> {
    validate_alert_fields(&alert.symbol, alert.threshold, &alert.currency, &alert.note)?;
    let pool = &db.0;
    db::insert_alert(pool, alert).await.map_err(AppError::from)
}

#[tauri::command]
pub async fn delete_alert(db: State<'_, DbState>, id: AlertId) -> Result<bool, AppError> {
    validate_id("alert ID", &id.0)?;
    let pool = &db.0;
    db::delete_alert(pool, &id).await.map_err(AppError::from)
}

#[tauri::command]
pub async fn reset_alert(db: State<'_, DbState>, id: AlertId) -> Result<bool, AppError> {
    validate_id("alert ID", &id.0)?;
    let pool = &db.0;
    db::reset_alert(pool, &id).await.map_err(AppError::from)
}
