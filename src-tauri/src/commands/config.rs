use tauri::State;

use crate::db;
use crate::error::AppError;

use super::{DbState, RealizedGainsCacheState};

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
