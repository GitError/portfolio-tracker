use tauri::State;

use crate::db;
use crate::error::AppError;

use super::{DbState, RealizedGainsCacheState};

/// Config keys readable/writable via `get_config_cmd`/`set_config_cmd`. Shared
/// by both commands so a key can never be read without also being writable
/// (or vice versa) — add new config keys here before wiring up a new setting.
const ALLOWED_CONFIG_KEYS: &[&str] = &[
    "base_currency",
    "app_language",
    "app_theme",
    "auto_refresh_interval_ms",
    "auto_refresh_market_hours_only",
    "cost_basis_method",
    "notifications_enabled",
    "holdings_hidden_columns",
];

fn validate_config_key(key: &str) -> Result<(), AppError> {
    if !ALLOWED_CONFIG_KEYS.contains(&key) {
        return Err(AppError::Validation(format!("Unknown config key: {key}")));
    }
    Ok(())
}

#[tauri::command]
pub async fn get_config_cmd(
    db: State<'_, DbState>,
    key: String,
) -> Result<Option<String>, AppError> {
    validate_config_key(&key)?;
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
    validate_config_key(&key)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_config_key_accepts_every_allowed_key() {
        for key in ALLOWED_CONFIG_KEYS {
            assert!(
                validate_config_key(key).is_ok(),
                "{key} should be a valid config key"
            );
        }
    }

    #[test]
    fn validate_config_key_rejects_unknown_key() {
        // Regression guard for #644: get_config_cmd previously had no
        // allowlist at all, so any key name — including internal/sensitive
        // ones — could be queried.
        let result = validate_config_key("some_internal_secret");
        assert!(result.is_err(), "unknown config key must be rejected");
        match result {
            Err(AppError::Validation(msg)) => {
                assert!(msg.contains("some_internal_secret"));
            }
            other => panic!("expected Validation error, got {other:?}"),
        }
    }

    #[test]
    fn validate_config_key_rejects_empty_key() {
        assert!(validate_config_key("").is_err());
    }

    #[test]
    fn validate_config_key_accepts_holdings_hidden_columns() {
        // Regression guard for #661: the frontend persists which Holdings
        // table columns are hidden under this key, but it was missing from
        // the allowlist, so get/set_config_cmd rejected it.
        assert!(validate_config_key("holdings_hidden_columns").is_ok());
    }
}
