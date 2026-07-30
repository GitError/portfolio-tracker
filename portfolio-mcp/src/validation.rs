//! Input validation for MCP write tools.
//!
//! The MCP server writes directly to the SQLite database, bypassing the
//! Tauri command layer entirely. These checks mirror the validation enforced
//! by the corresponding Tauri commands (see `src-tauri/src/commands/mod.rs`,
//! `commands/alerts.rs`, `commands/transactions.rs`, and `commands/config.rs`)
//! so the MCP layer cannot be used to write data the desktop app itself would
//! reject.

use rmcp::Error as McpError;

/// Mirrors `validate_holding_fields` in `src-tauri/src/commands/mod.rs`.
/// Returns the normalized (uppercase, trimmed) currency code.
pub fn validate_holding_fields(
    quantity: f64,
    cost_basis: f64,
    currency: &str,
) -> Result<String, McpError> {
    if quantity <= 0.0 || !quantity.is_finite() {
        return Err(McpError::invalid_params(
            "quantity must be a positive finite number",
            None,
        ));
    }
    if cost_basis < 0.0 || !cost_basis.is_finite() {
        return Err(McpError::invalid_params(
            "costBasis must be a non-negative finite number",
            None,
        ));
    }
    let currency = currency.trim().to_uppercase();
    if currency.len() != 3 || !currency.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(McpError::invalid_params(
            "currency must be a 3-letter ISO currency code",
            None,
        ));
    }
    Ok(currency)
}

/// Rejects a blank/whitespace-only required string field.
pub fn validate_non_empty(field: &str, value: &str) -> Result<(), McpError> {
    if value.trim().is_empty() {
        return Err(McpError::invalid_params(
            format!("{field} must not be empty"),
            None,
        ));
    }
    Ok(())
}

/// Mirrors the `target_weight` bounds check applied by the CSV import layer
/// (`src-tauri/src/csv.rs`) and implicitly required by `add_holding`/`update_holding`.
pub fn validate_target_weight(target_weight: Option<f64>) -> Result<(), McpError> {
    if let Some(weight) = target_weight {
        if !weight.is_finite() || !(0.0..=100.0).contains(&weight) {
            return Err(McpError::invalid_params(
                "targetWeight must be a finite number between 0 and 100",
                None,
            ));
        }
    }
    Ok(())
}

/// Mirrors the portfolio-wide target-weight guard in `commands/portfolio.rs`:
/// the sum of all target weights (excluding the holding being edited) plus the
/// new value must not exceed 100%.
pub fn validate_target_weight_budget(
    target_weight: Option<f64>,
    existing_sum: f64,
) -> Result<(), McpError> {
    const WEIGHT_EPSILON: f64 = 0.001;
    if let Some(weight) = target_weight {
        if weight > 0.0 && existing_sum + weight > 100.0 + WEIGHT_EPSILON {
            return Err(McpError::invalid_params(
                format!(
                    "Total target weight would exceed 100% (currently {existing_sum:.1}%). \
                     Adjust existing allocations before adding this holding."
                ),
                None,
            ));
        }
    }
    Ok(())
}

/// Mirrors the checks in `src-tauri/src/commands/transactions.rs::add_transaction`.
pub fn validate_transaction_fields(quantity: f64, price: f64) -> Result<(), McpError> {
    if quantity <= 0.0 || !quantity.is_finite() {
        return Err(McpError::invalid_params(
            "quantity must be a positive finite number",
            None,
        ));
    }
    if price < 0.0 || !price.is_finite() {
        return Err(McpError::invalid_params(
            "price must be a non-negative finite number",
            None,
        ));
    }
    Ok(())
}

/// Mirrors the check in `src-tauri/src/commands/alerts.rs::add_alert`.
pub fn validate_alert_threshold(threshold: f64) -> Result<(), McpError> {
    if !threshold.is_finite() || threshold <= 0.0 {
        return Err(McpError::invalid_params(
            "threshold must be a positive finite number",
            None,
        ));
    }
    Ok(())
}

/// Config keys the Tauri `set_config_cmd` allowlist accepts
/// (`src-tauri/src/commands/config.rs::ALLOWED_CONFIG_KEYS`). Kept in sync
/// manually since the MCP server does not depend on the `src-tauri` crate.
pub const ALLOWED_CONFIG_KEYS: &[&str] = &[
    "base_currency",
    "app_language",
    "app_theme",
    "auto_refresh_interval_ms",
    "auto_refresh_market_hours_only",
    "cost_basis_method",
    "notifications_enabled",
];

/// Mirrors the allowlist check in `src-tauri/src/commands/config.rs::set_config_cmd`.
pub fn validate_config_key(key: &str) -> Result<(), McpError> {
    if !ALLOWED_CONFIG_KEYS.contains(&key) {
        return Err(McpError::invalid_params(
            format!("Unknown config key: {key}"),
            None,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_holding_fields_rejects_nan_quantity() {
        let err = validate_holding_fields(f64::NAN, 100.0, "USD");
        assert!(err.is_err(), "NaN quantity must be rejected");
    }

    #[test]
    fn validate_holding_fields_rejects_infinite_cost_basis() {
        let err = validate_holding_fields(1.0, f64::INFINITY, "USD");
        assert!(err.is_err(), "infinite cost_basis must be rejected");
    }

    #[test]
    fn validate_holding_fields_rejects_non_positive_quantity() {
        assert!(validate_holding_fields(0.0, 100.0, "USD").is_err());
        assert!(validate_holding_fields(-5.0, 100.0, "USD").is_err());
    }

    #[test]
    fn validate_holding_fields_rejects_negative_cost_basis() {
        assert!(validate_holding_fields(1.0, -1.0, "USD").is_err());
    }

    #[test]
    fn validate_holding_fields_rejects_malformed_currency() {
        assert!(validate_holding_fields(1.0, 100.0, "US").is_err());
        assert!(validate_holding_fields(1.0, 100.0, "USDD").is_err());
        assert!(validate_holding_fields(1.0, 100.0, "U5D").is_err());
    }

    #[test]
    fn validate_holding_fields_normalizes_currency_case_and_whitespace() {
        let currency = validate_holding_fields(1.0, 100.0, " usd ").expect("valid input");
        assert_eq!(currency, "USD");
    }

    #[test]
    fn validate_holding_fields_accepts_valid_input() {
        assert!(validate_holding_fields(10.0, 150.25, "CAD").is_ok());
    }

    #[test]
    fn validate_non_empty_rejects_blank_and_whitespace() {
        assert!(validate_non_empty("symbol", "").is_err());
        assert!(validate_non_empty("symbol", "   ").is_err());
        assert!(validate_non_empty("symbol", "AAPL").is_ok());
    }

    #[test]
    fn validate_target_weight_rejects_out_of_range_and_nan() {
        assert!(validate_target_weight(Some(-1.0)).is_err());
        assert!(validate_target_weight(Some(100.001)).is_err());
        assert!(validate_target_weight(Some(f64::NAN)).is_err());
        assert!(validate_target_weight(Some(50.0)).is_ok());
        assert!(validate_target_weight(None).is_ok());
    }

    #[test]
    fn validate_target_weight_budget_rejects_when_total_exceeds_100() {
        assert!(validate_target_weight_budget(Some(50.0), 60.0).is_err());
        assert!(validate_target_weight_budget(Some(40.0), 60.0).is_ok());
        // Non-positive weights don't consume budget and are always allowed here.
        assert!(validate_target_weight_budget(Some(0.0), 99.0).is_ok());
        assert!(validate_target_weight_budget(None, 99.0).is_ok());
    }

    #[test]
    fn validate_transaction_fields_rejects_nan_and_non_positive_quantity() {
        assert!(validate_transaction_fields(f64::NAN, 10.0).is_err());
        assert!(validate_transaction_fields(0.0, 10.0).is_err());
        assert!(validate_transaction_fields(-1.0, 10.0).is_err());
    }

    #[test]
    fn validate_transaction_fields_rejects_negative_or_infinite_price() {
        assert!(validate_transaction_fields(1.0, -1.0).is_err());
        assert!(validate_transaction_fields(1.0, f64::INFINITY).is_err());
    }

    #[test]
    fn validate_transaction_fields_accepts_valid_input() {
        assert!(validate_transaction_fields(5.0, 0.0).is_ok());
        assert!(validate_transaction_fields(5.0, 120.5).is_ok());
    }

    #[test]
    fn validate_alert_threshold_rejects_nan_and_non_positive() {
        assert!(validate_alert_threshold(f64::NAN).is_err());
        assert!(validate_alert_threshold(0.0).is_err());
        assert!(validate_alert_threshold(-10.0).is_err());
        assert!(validate_alert_threshold(f64::INFINITY).is_err());
    }

    #[test]
    fn validate_alert_threshold_accepts_positive_finite() {
        assert!(validate_alert_threshold(150.0).is_ok());
    }

    #[test]
    fn validate_config_key_rejects_unknown_keys() {
        assert!(validate_config_key("base_currency").is_ok());
        assert!(validate_config_key("cost_basis_method").is_ok());
        assert!(validate_config_key("some_arbitrary_key").is_err());
        assert!(validate_config_key("").is_err());
    }
}
