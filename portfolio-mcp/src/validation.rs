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

/// Mirrors `validate_id` in `src-tauri/src/commands/mod.rs`. All IDs in this app
/// are UUID v4 strings generated via `uuid::Uuid::new_v4()`; a malformed ID would
/// otherwise silently no-op in SQLite (0 rows affected) instead of surfacing a
/// clear error (see #685).
pub fn validate_id(field: &str, id: &str) -> Result<(), McpError> {
    if id.trim().is_empty() || uuid::Uuid::parse_str(id.trim()).is_err() {
        return Err(McpError::invalid_params(format!("Invalid {field}"), None));
    }
    Ok(())
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

/// Dividend frequencies accepted by the CSV import layer (`src-tauri/src/csv.rs`),
/// mirrored here so the MCP `add_holding` tool applies the same set.
pub const VALID_DIVIDEND_FREQUENCIES: &[&str] =
    &["monthly", "quarterly", "semi-annual", "annual", "irregular"];

/// Mirrors `validate_holding_dividend_fields` in `src-tauri/src/commands/mod.rs`.
pub fn validate_holding_dividend_fields(
    indicated_annual_dividend: Option<f64>,
    dividend_frequency: Option<&str>,
    maturity_date: Option<&str>,
) -> Result<(), McpError> {
    if let Some(amount) = indicated_annual_dividend {
        if !amount.is_finite() || amount < 0.0 {
            return Err(McpError::invalid_params(
                "indicatedAnnualDividend must be a non-negative finite number",
                None,
            ));
        }
    }
    if let Some(freq) = dividend_frequency {
        let normalized = freq.trim().to_lowercase();
        if !VALID_DIVIDEND_FREQUENCIES.contains(&normalized.as_str()) {
            return Err(McpError::invalid_params(
                format!(
                    "dividendFrequency must be one of: {}",
                    VALID_DIVIDEND_FREQUENCIES.join(", ")
                ),
                None,
            ));
        }
    }
    if let Some(date) = maturity_date {
        if chrono::NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d").is_err() {
            return Err(McpError::invalid_params(
                "maturityDate must be a valid ISO date (YYYY-MM-DD)",
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

/// Max length for a price alert's free-text note, to prevent abuse.
/// Mirrors `MAX_ALERT_NOTE_LEN` in `src-tauri/src/commands/mod.rs`.
pub const MAX_ALERT_NOTE_LEN: usize = 500;

/// Mirrors the currency check in `src-tauri/src/commands/mod.rs::validate_alert_fields`.
pub fn validate_alert_currency(currency: &str) -> Result<(), McpError> {
    let currency = currency.trim();
    if !(2..=3).contains(&currency.len()) || !currency.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(McpError::invalid_params(
            "currency must be 2-3 uppercase letters",
            None,
        ));
    }
    Ok(())
}

/// Mirrors the note-length check in `src-tauri/src/commands/mod.rs::validate_alert_fields`.
pub fn validate_alert_note(note: &str) -> Result<(), McpError> {
    if note.chars().count() > MAX_ALERT_NOTE_LEN {
        return Err(McpError::invalid_params(
            format!("note must be at most {MAX_ALERT_NOTE_LEN} characters"),
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
    "holdings_hidden_columns",
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

/// Mirrors `SUPPORTED_LANGUAGES` in `src-tauri/src/commands/config.rs`.
const SUPPORTED_LANGUAGES: &[&str] = &["en", "de", "es", "fr", "ja", "pl", "pt", "zh"];

/// Mirrors `KNOWN_HOLDINGS_COLUMNS` in `src-tauri/src/commands/config.rs`.
const KNOWN_HOLDINGS_COLUMNS: &[&str] = &[
    "symbol",
    "name",
    "assetType",
    "account",
    "exchange",
    "quantity",
    "costBasis",
    "currentPrice",
    "marketValueCad",
    "weight",
    "targetWeight",
    "targetDeltaPercent",
    "targetDeltaValue",
    "gainLoss",
    "gainLossPercent",
    "prevClose",
    "dayOpen",
    "openDate",
    "maturityDate",
];

/// Mirrors `MAX_CONFIG_VALUE_LEN` in `src-tauri/src/commands/config.rs`.
const MAX_CONFIG_VALUE_LEN: usize = 500;

/// Account types accepted by `create_account`, mirrors `VALID_ACCOUNT_TYPES`
/// in `src-tauri/src/commands/accounts.rs`.
pub const VALID_ACCOUNT_TYPES: &[&str] =
    &["tfsa", "rrsp", "fhsa", "taxable", "crypto", "cash", "other"];

/// Mirrors `validate_account_fields` in `src-tauri/src/commands/accounts.rs`.
/// Returns the trimmed name on success.
pub fn validate_account_fields(name: &str, account_type: &str) -> Result<String, McpError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(McpError::invalid_params(
            "Account name cannot be empty",
            None,
        ));
    }
    if !VALID_ACCOUNT_TYPES.contains(&account_type) {
        return Err(McpError::invalid_params(
            format!("Invalid account type: {account_type}"),
            None,
        ));
    }
    Ok(name)
}

/// Mirrors `validate_dividend_fields` in `src-tauri/src/commands/mod.rs`.
pub fn validate_dividend_fields(
    amount_per_unit: f64,
    ex_date: &str,
    pay_date: &str,
) -> Result<(), McpError> {
    if !amount_per_unit.is_finite() || amount_per_unit <= 0.0 {
        return Err(McpError::invalid_params(
            "amountPerUnit must be a finite number greater than 0",
            None,
        ));
    }
    if chrono::NaiveDate::parse_from_str(ex_date.trim(), "%Y-%m-%d").is_err() {
        return Err(McpError::invalid_params(
            "exDate must be a valid ISO date (YYYY-MM-DD)",
            None,
        ));
    }
    if chrono::NaiveDate::parse_from_str(pay_date.trim(), "%Y-%m-%d").is_err() {
        return Err(McpError::invalid_params(
            "payDate must be a valid ISO date (YYYY-MM-DD)",
            None,
        ));
    }
    if pay_date < ex_date {
        return Err(McpError::invalid_params(
            "payDate must not be before exDate",
            None,
        ));
    }
    Ok(())
}

/// Mirrors `validate_config_value` in `src-tauri/src/commands/config.rs::set_config_cmd`.
pub fn validate_config_value(key: &str, value: &str) -> Result<(), McpError> {
    match key {
        "app_theme" => {
            if !["light", "dark", "system"].contains(&value) {
                return Err(McpError::invalid_params(
                    format!("app_theme must be one of: light, dark, system (got: {value})"),
                    None,
                ));
            }
        }
        "app_language" => {
            if !SUPPORTED_LANGUAGES.contains(&value) {
                return Err(McpError::invalid_params(
                    format!(
                        "app_language must be one of: {} (got: {value})",
                        SUPPORTED_LANGUAGES.join(", ")
                    ),
                    None,
                ));
            }
        }
        "base_currency" => {
            if value.len() != 3 || !value.bytes().all(|b| b.is_ascii_uppercase()) {
                return Err(McpError::invalid_params(
                    format!(
                        "base_currency must be a 3-letter uppercase currency code (got: {value})"
                    ),
                    None,
                ));
            }
        }
        "holdings_hidden_columns" => {
            let columns: Vec<String> = serde_json::from_str(value).map_err(|_| {
                McpError::invalid_params(
                    "holdings_hidden_columns must be a JSON array of column names",
                    None,
                )
            })?;
            if let Some(unknown) = columns
                .iter()
                .find(|c| !KNOWN_HOLDINGS_COLUMNS.contains(&c.as_str()))
            {
                return Err(McpError::invalid_params(
                    format!("holdings_hidden_columns contains unknown column: {unknown}"),
                    None,
                ));
            }
        }
        "cost_basis_method" => {
            if !value.eq_ignore_ascii_case("avco") && !value.eq_ignore_ascii_case("fifo") {
                return Err(McpError::invalid_params(
                    format!("cost_basis_method must be one of: avco, fifo (got: {value})"),
                    None,
                ));
            }
        }
        "notifications_enabled" | "auto_refresh_market_hours_only" => {
            if !["true", "false"].contains(&value) {
                return Err(McpError::invalid_params(
                    format!("{key} must be one of: true, false (got: {value})"),
                    None,
                ));
            }
        }
        _ => {
            if value.len() > MAX_CONFIG_VALUE_LEN {
                return Err(McpError::invalid_params(
                    format!("{key} value exceeds max length of {MAX_CONFIG_VALUE_LEN} characters"),
                    None,
                ));
            }
        }
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
    fn validate_alert_currency_rejects_malformed() {
        assert!(validate_alert_currency("").is_err());
        assert!(validate_alert_currency("A").is_err());
        assert!(validate_alert_currency("ABCD").is_err());
        assert!(validate_alert_currency("usd").is_err());
        assert!(validate_alert_currency("U5D").is_err());
    }

    #[test]
    fn validate_alert_currency_accepts_valid() {
        assert!(validate_alert_currency("CAD").is_ok());
        assert!(validate_alert_currency("US").is_ok());
    }

    #[test]
    fn validate_alert_note_rejects_over_max_length() {
        let note = "a".repeat(MAX_ALERT_NOTE_LEN + 1);
        assert!(validate_alert_note(&note).is_err());
    }

    #[test]
    fn validate_alert_note_accepts_within_max_length() {
        let note = "a".repeat(MAX_ALERT_NOTE_LEN);
        assert!(validate_alert_note(&note).is_ok());
        assert!(validate_alert_note("").is_ok());
    }

    #[test]
    fn validate_holding_dividend_fields_rejects_nan_indicated_annual_dividend() {
        assert!(validate_holding_dividend_fields(Some(f64::NAN), None, None).is_err());
    }

    #[test]
    fn validate_holding_dividend_fields_rejects_infinite_indicated_annual_dividend() {
        assert!(validate_holding_dividend_fields(Some(f64::INFINITY), None, None).is_err());
    }

    #[test]
    fn validate_holding_dividend_fields_rejects_negative_indicated_annual_dividend() {
        assert!(validate_holding_dividend_fields(Some(-1.0), None, None).is_err());
    }

    #[test]
    fn validate_holding_dividend_fields_accepts_zero_indicated_annual_dividend() {
        assert!(validate_holding_dividend_fields(Some(0.0), None, None).is_ok());
    }

    #[test]
    fn validate_holding_dividend_fields_rejects_unknown_dividend_frequency() {
        assert!(validate_holding_dividend_fields(None, Some("biannual"), None).is_err());
    }

    #[test]
    fn validate_holding_dividend_fields_accepts_known_dividend_frequencies() {
        for freq in ["monthly", "quarterly", "semi-annual", "annual", "irregular"] {
            assert!(
                validate_holding_dividend_fields(None, Some(freq), None).is_ok(),
                "expected {freq} to be accepted"
            );
        }
    }

    #[test]
    fn validate_holding_dividend_fields_rejects_non_iso_maturity_date() {
        assert!(validate_holding_dividend_fields(None, None, Some("01/30/2030")).is_err());
    }

    #[test]
    fn validate_holding_dividend_fields_accepts_iso_maturity_date() {
        assert!(validate_holding_dividend_fields(None, None, Some("2030-01-01")).is_ok());
    }

    #[test]
    fn validate_holding_dividend_fields_accepts_all_none() {
        assert!(validate_holding_dividend_fields(None, None, None).is_ok());
    }

    #[test]
    fn validate_id_rejects_empty_string() {
        assert!(validate_id("holding ID", "").is_err());
    }

    #[test]
    fn validate_id_rejects_whitespace_only() {
        assert!(validate_id("holding ID", "   ").is_err());
    }

    #[test]
    fn validate_id_rejects_malformed_uuid() {
        assert!(validate_id("holding ID", "not-a-uuid").is_err());
        assert!(validate_id("holding ID", "12345").is_err());
    }

    #[test]
    fn validate_id_accepts_valid_uuid() {
        assert!(validate_id("holding ID", "550e8400-e29b-41d4-a716-446655440000").is_ok());
    }

    #[test]
    fn validate_id_accepts_uuid_with_surrounding_whitespace() {
        assert!(validate_id("holding ID", "  550e8400-e29b-41d4-a716-446655440000  ").is_ok());
    }

    #[test]
    fn validate_config_key_rejects_unknown_keys() {
        assert!(validate_config_key("base_currency").is_ok());
        assert!(validate_config_key("cost_basis_method").is_ok());
        assert!(validate_config_key("some_arbitrary_key").is_err());
        assert!(validate_config_key("").is_err());
    }

    #[test]
    fn validate_config_value_accepts_known_theme_values() {
        for value in ["light", "dark", "system"] {
            assert!(validate_config_value("app_theme", value).is_ok());
        }
    }

    #[test]
    fn validate_config_value_rejects_unknown_theme() {
        assert!(validate_config_value("app_theme", "solarized").is_err());
    }

    #[test]
    fn validate_config_value_accepts_supported_languages() {
        for value in ["en", "de", "es", "fr", "ja", "pl", "pt", "zh"] {
            assert!(validate_config_value("app_language", value).is_ok());
        }
    }

    #[test]
    fn validate_config_value_rejects_unsupported_language() {
        assert!(validate_config_value("app_language", "xx").is_err());
    }

    #[test]
    fn validate_config_value_accepts_valid_base_currency() {
        assert!(validate_config_value("base_currency", "CAD").is_ok());
    }

    #[test]
    fn validate_config_value_rejects_malformed_base_currency() {
        assert!(validate_config_value("base_currency", "cad").is_err());
        assert!(validate_config_value("base_currency", "CA").is_err());
    }

    #[test]
    fn validate_config_value_accepts_valid_holdings_hidden_columns() {
        assert!(validate_config_value("holdings_hidden_columns", r#"["symbol","weight"]"#).is_ok());
    }

    #[test]
    fn validate_config_value_rejects_unknown_column_in_holdings_hidden_columns() {
        assert!(validate_config_value("holdings_hidden_columns", r#"["notARealColumn"]"#).is_err());
    }

    #[test]
    fn validate_config_value_rejects_non_json_holdings_hidden_columns() {
        assert!(validate_config_value("holdings_hidden_columns", "not json").is_err());
    }

    #[test]
    fn validate_config_value_accepts_other_keys_within_max_length() {
        assert!(validate_config_value("auto_refresh_interval_ms", "60000").is_ok());
    }

    #[test]
    fn validate_config_value_rejects_other_keys_over_max_length() {
        let too_long = "a".repeat(MAX_CONFIG_VALUE_LEN + 1);
        assert!(validate_config_value("auto_refresh_interval_ms", &too_long).is_err());
    }

    #[test]
    fn validate_config_value_accepts_avco_and_fifo_case_insensitively() {
        // Regression guard for #714: cost_basis_method previously fell through
        // to the generic max-length check, so any string was accepted.
        for value in ["avco", "fifo", "AVCO", "FIFO", "Avco", "Fifo"] {
            assert!(
                validate_config_value("cost_basis_method", value).is_ok(),
                "{value} should be accepted"
            );
        }
    }

    #[test]
    fn validate_config_value_rejects_invalid_cost_basis_method() {
        assert!(validate_config_value("cost_basis_method", "fefo").is_err());
        assert!(validate_config_value("cost_basis_method", "").is_err());
    }

    #[test]
    fn validate_account_fields_rejects_empty_and_whitespace_name() {
        assert!(validate_account_fields("", "tfsa").is_err());
        assert!(validate_account_fields("   ", "tfsa").is_err());
    }

    #[test]
    fn validate_account_fields_rejects_unknown_type() {
        assert!(validate_account_fields("My Account", "not-a-type").is_err());
    }

    #[test]
    fn validate_account_fields_trims_name_and_accepts_valid_input() {
        let name = validate_account_fields("  My TFSA  ", "tfsa").expect("valid input");
        assert_eq!(name, "My TFSA");
    }

    #[test]
    fn validate_dividend_fields_rejects_non_positive_amount() {
        assert!(validate_dividend_fields(0.0, "2024-01-01", "2024-01-15").is_err());
        assert!(validate_dividend_fields(-1.0, "2024-01-01", "2024-01-15").is_err());
    }

    #[test]
    fn validate_dividend_fields_rejects_non_finite_amount() {
        assert!(validate_dividend_fields(f64::NAN, "2024-01-01", "2024-01-15").is_err());
        assert!(validate_dividend_fields(f64::INFINITY, "2024-01-01", "2024-01-15").is_err());
    }

    #[test]
    fn validate_dividend_fields_rejects_non_iso_dates() {
        assert!(validate_dividend_fields(1.0, "01/01/2024", "2024-01-15").is_err());
        assert!(validate_dividend_fields(1.0, "2024-01-01", "01/15/2024").is_err());
    }

    #[test]
    fn validate_dividend_fields_rejects_pay_date_before_ex_date() {
        assert!(validate_dividend_fields(1.0, "2024-01-15", "2024-01-01").is_err());
    }

    #[test]
    fn validate_dividend_fields_accepts_valid_input() {
        assert!(validate_dividend_fields(1.5, "2024-01-01", "2024-01-15").is_ok());
        assert!(validate_dividend_fields(1.5, "2024-01-01", "2024-01-01").is_ok());
    }
}
