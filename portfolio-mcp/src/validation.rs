//! Input validation for MCP write tools.
//!
//! The MCP server writes directly to the SQLite database, bypassing the
//! Tauri command layer entirely. The actual rules live in
//! `portfolio_core::validation` (shared with `src-tauri/src/commands/`, see
//! #758) so the MCP layer cannot be used to write data the desktop app itself
//! would reject, and a rule change in one place can't drift out of sync with
//! the other. Every function here is a thin adapter: it calls the shared
//! rule and converts the `Result<T, String>` it returns into the `McpError`
//! shape the rmcp tool handlers expect, preserving the exact message text.

use rmcp::Error as McpError;

fn invalid(message: String) -> McpError {
    McpError::invalid_params(message, None)
}

/// Mirrors `validate_holding_fields` in `src-tauri/src/commands/mod.rs`.
/// Returns the normalized (uppercase, trimmed) currency code.
pub fn validate_holding_fields(
    quantity: f64,
    cost_basis: f64,
    currency: &str,
) -> Result<String, McpError> {
    portfolio_core::validation::validate_holding_fields(quantity, cost_basis, currency)
        .map_err(invalid)
}

/// Mirrors `validate_id` in `src-tauri/src/commands/mod.rs`.
pub fn validate_id(field: &str, id: &str) -> Result<(), McpError> {
    portfolio_core::validation::validate_id(field, id).map_err(invalid)
}

/// Rejects a blank/whitespace-only required string field.
pub fn validate_non_empty(field: &str, value: &str) -> Result<(), McpError> {
    portfolio_core::validation::validate_non_empty(field, value).map_err(invalid)
}

/// Mirrors the `target_weight` bounds check applied by the CSV import layer
/// (`src-tauri/src/csv.rs`) and implicitly required by `add_holding`/`update_holding`.
pub fn validate_target_weight(target_weight: Option<f64>) -> Result<(), McpError> {
    portfolio_core::validation::validate_target_weight(target_weight).map_err(invalid)
}

/// Mirrors the portfolio-wide target-weight guard in `commands/portfolio.rs`:
/// the sum of all target weights (excluding the holding being edited) plus the
/// new value must not exceed 100%.
pub fn validate_target_weight_budget(
    target_weight: Option<f64>,
    existing_sum: f64,
) -> Result<(), McpError> {
    if let Some(weight) = target_weight {
        if portfolio_core::validation::exceeds_target_weight_budget(weight, existing_sum) {
            return Err(invalid(format!(
                "Total target weight would exceed 100% (currently {existing_sum:.1}%). \
                 Adjust existing allocations before adding this holding."
            )));
        }
    }
    Ok(())
}

/// Mirrors `validate_holding_dividend_fields` in `src-tauri/src/commands/mod.rs`.
pub fn validate_holding_dividend_fields(
    indicated_annual_dividend: Option<f64>,
    dividend_frequency: Option<&str>,
    maturity_date: Option<&str>,
) -> Result<(), McpError> {
    portfolio_core::validation::validate_holding_dividend_fields(
        indicated_annual_dividend,
        dividend_frequency,
        maturity_date,
    )
    .map_err(invalid)
}

/// Mirrors the checks in `src-tauri/src/commands/transactions.rs::add_transaction`.
pub fn validate_transaction_fields(quantity: f64, price: f64) -> Result<(), McpError> {
    portfolio_core::validation::validate_transaction_fields(quantity, price).map_err(invalid)
}

/// Mirrors the check in `src-tauri/src/commands/alerts.rs::add_alert`.
pub fn validate_alert_threshold(threshold: f64) -> Result<(), McpError> {
    portfolio_core::validation::validate_alert_threshold(threshold).map_err(invalid)
}

/// Mirrors the currency check in `src-tauri/src/commands/mod.rs::validate_alert_fields`.
pub fn validate_alert_currency(currency: &str) -> Result<(), McpError> {
    portfolio_core::validation::validate_alert_currency(currency).map_err(invalid)
}

/// Mirrors the note-length check in `src-tauri/src/commands/mod.rs::validate_alert_fields`.
pub fn validate_alert_note(note: &str) -> Result<(), McpError> {
    portfolio_core::validation::validate_alert_note(note).map_err(invalid)
}

/// Mirrors the allowlist check in `src-tauri/src/commands/config.rs::set_config_cmd`.
pub fn validate_config_key(key: &str) -> Result<(), McpError> {
    portfolio_core::validation::validate_config_key(key).map_err(invalid)
}

/// Mirrors `validate_account_fields` in `src-tauri/src/commands/accounts.rs`.
/// Returns the trimmed name on success.
pub fn validate_account_fields(name: &str, account_type: &str) -> Result<String, McpError> {
    portfolio_core::validation::validate_account_fields(name, account_type).map_err(invalid)
}

/// Mirrors `validate_dividend_fields` in `src-tauri/src/commands/mod.rs`.
pub fn validate_dividend_fields(
    amount_per_unit: f64,
    ex_date: &str,
    pay_date: &str,
) -> Result<(), McpError> {
    portfolio_core::validation::validate_dividend_fields(amount_per_unit, ex_date, pay_date)
        .map_err(invalid)
}

/// Mirrors `validate_config_value` in `src-tauri/src/commands/config.rs::set_config_cmd`.
pub fn validate_config_value(key: &str, value: &str) -> Result<(), McpError> {
    portfolio_core::validation::validate_config_value(key, value).map_err(invalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests exercise the McpError adaptation at this layer (message
    // text, invalid_params variant), not the underlying rules — those are
    // fully covered by portfolio-core/src/validation/*. A representative
    // invalid/valid pair per function is enough to prove the adapter works;
    // exhaustive edge cases live with the shared rule.

    #[test]
    fn validate_holding_fields_adapts_rejection_to_invalid_params() {
        let err = validate_holding_fields(0.0, 100.0, "USD").expect_err("must reject");
        assert!(err.message.contains("quantity"));
    }

    #[test]
    fn validate_holding_fields_normalizes_currency_case_and_whitespace() {
        let currency = validate_holding_fields(1.0, 100.0, " usd ").expect("valid input");
        assert_eq!(currency, "USD");
    }

    #[test]
    fn validate_id_rejects_malformed_uuid() {
        assert!(validate_id("holding ID", "not-a-uuid").is_err());
    }

    #[test]
    fn validate_id_accepts_valid_uuid() {
        assert!(validate_id("holding ID", "550e8400-e29b-41d4-a716-446655440000").is_ok());
    }

    #[test]
    fn validate_non_empty_rejects_blank() {
        assert!(validate_non_empty("symbol", "   ").is_err());
        assert!(validate_non_empty("symbol", "AAPL").is_ok());
    }

    #[test]
    fn validate_target_weight_rejects_out_of_range() {
        assert!(validate_target_weight(Some(150.0)).is_err());
        assert!(validate_target_weight(Some(50.0)).is_ok());
    }

    #[test]
    fn validate_target_weight_budget_rejects_when_total_exceeds_100() {
        let err = validate_target_weight_budget(Some(50.0), 60.0).expect_err("must reject");
        assert!(err.message.contains("exceed 100%"));
        assert!(validate_target_weight_budget(Some(40.0), 60.0).is_ok());
        // Non-positive weights don't consume budget and are always allowed here.
        assert!(validate_target_weight_budget(Some(0.0), 99.0).is_ok());
        assert!(validate_target_weight_budget(None, 99.0).is_ok());
    }

    #[test]
    fn validate_holding_dividend_fields_rejects_unknown_frequency() {
        assert!(validate_holding_dividend_fields(None, Some("biannual"), None).is_err());
        assert!(validate_holding_dividend_fields(None, Some("monthly"), None).is_ok());
    }

    #[test]
    fn validate_transaction_fields_rejects_non_positive_quantity() {
        assert!(validate_transaction_fields(0.0, 10.0).is_err());
        assert!(validate_transaction_fields(5.0, 120.5).is_ok());
    }

    #[test]
    fn validate_alert_threshold_rejects_non_positive() {
        assert!(validate_alert_threshold(0.0).is_err());
        assert!(validate_alert_threshold(150.0).is_ok());
    }

    #[test]
    fn validate_alert_currency_rejects_malformed() {
        assert!(validate_alert_currency("usd").is_err());
        assert!(validate_alert_currency("CAD").is_ok());
    }

    #[test]
    fn validate_alert_note_rejects_over_max_length() {
        let note = "a".repeat(portfolio_core::validation::MAX_ALERT_NOTE_LEN + 1);
        assert!(validate_alert_note(&note).is_err());
        assert!(validate_alert_note("").is_ok());
    }

    #[test]
    fn validate_config_key_rejects_unknown_keys() {
        assert!(validate_config_key("base_currency").is_ok());
        assert!(validate_config_key("some_arbitrary_key").is_err());
    }

    #[test]
    fn validate_config_value_rejects_unknown_theme() {
        assert!(validate_config_value("app_theme", "solarized").is_err());
        assert!(validate_config_value("app_theme", "dark").is_ok());
    }

    #[test]
    fn validate_config_value_accepts_avco_and_fifo_case_insensitively() {
        // Regression guard for #714: cost_basis_method previously fell through
        // to the generic max-length check, so any string was accepted.
        for value in ["avco", "fifo", "AVCO", "FIFO"] {
            assert!(
                validate_config_value("cost_basis_method", value).is_ok(),
                "{value} should be accepted"
            );
        }
        assert!(validate_config_value("cost_basis_method", "fefo").is_err());
    }

    #[test]
    fn validate_account_fields_rejects_empty_name_and_trims_valid_input() {
        assert!(validate_account_fields("   ", "tfsa").is_err());
        assert!(validate_account_fields("not-a-type", "not-a-type").is_err());
        let name = validate_account_fields("  My TFSA  ", "tfsa").expect("valid input");
        assert_eq!(name, "My TFSA");
    }

    #[test]
    fn validate_dividend_fields_rejects_pay_date_before_ex_date() {
        assert!(validate_dividend_fields(1.0, "2024-01-15", "2024-01-01").is_err());
        assert!(validate_dividend_fields(1.5, "2024-01-01", "2024-01-15").is_ok());
    }

    // ── Cross-surface parity ────────────────────────────────────────────────
    // Regression guard for #758: both the Tauri command layer and this MCP
    // layer must reject the same representative invalid inputs, because both
    // now call the identical `portfolio_core::validation` rule.

    #[test]
    fn representative_invalid_inputs_are_rejected_identically_to_shared_core() {
        use portfolio_core::validation as core;

        assert_eq!(
            validate_holding_fields(-1.0, 100.0, "USD").is_err(),
            core::validate_holding_fields(-1.0, 100.0, "USD").is_err()
        );
        assert_eq!(
            validate_id("id", "not-a-uuid").is_err(),
            core::validate_id("id", "not-a-uuid").is_err()
        );
        assert_eq!(
            validate_transaction_fields(0.0, 10.0).is_err(),
            core::validate_transaction_fields(0.0, 10.0).is_err()
        );
        assert_eq!(
            validate_alert_currency("usd").is_err(),
            core::validate_alert_currency("usd").is_err()
        );
        assert_eq!(
            validate_config_key("some_internal_secret").is_err(),
            core::validate_config_key("some_internal_secret").is_err()
        );
        assert_eq!(
            validate_account_fields("", "tfsa").is_err(),
            core::validate_account_fields("", "tfsa").is_err()
        );
        assert_eq!(
            validate_dividend_fields(0.0, "2024-01-01", "2024-01-15").is_err(),
            core::validate_dividend_fields(0.0, "2024-01-01", "2024-01-15").is_err()
        );
    }
}
