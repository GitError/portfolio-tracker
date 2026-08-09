//! App config key/value validation — the `set_config`/`get_config` allowlist
//! and per-key value shape checks.

/// Config keys readable/writable via `get_config`/`set_config`. Shared by both
/// operations so a key can never be read without also being writable (or vice
/// versa) — add new config keys here before wiring up a new setting.
pub const ALLOWED_CONFIG_KEYS: &[&str] = &[
    "base_currency",
    "app_language",
    "app_theme",
    "app_color_scheme",
    "auto_refresh_interval_ms",
    "auto_refresh_market_hours_only",
    "cost_basis_method",
    "notifications_enabled",
    "holdings_hidden_columns",
];

/// Color scheme keys the frontend can render (`frontend/lib/themes.ts`'s
/// `COLOR_SCHEME_KEYS`), mirrored here so `app_color_scheme` can't be set to
/// a scheme the UI doesn't know how to draw.
const SUPPORTED_COLOR_SCHEMES: &[&str] = &["default", "dracula", "synthwave", "nord", "warmLight"];

/// BCP 47 tags this app ships translations for (`frontend/lib/i18n.ts`'s
/// `SUPPORTED_LNG_CODES`).
const SUPPORTED_LANGUAGES: &[&str] = &["en", "de", "es", "fr", "ja", "pl", "pt", "zh"];

/// Holdings table columns a user can hide (`frontend/components/Holdings.tsx`'s
/// `ALL_COLUMNS`), mirrored here so `holdings_hidden_columns` can't be set to
/// reference a column that doesn't exist.
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

/// Keys not otherwise recognized below fall through to this generic bound —
/// long enough for any legitimate config value, short enough to stop the
/// allowlisted-key check from being paired with an unbounded value.
pub const MAX_CONFIG_VALUE_LEN: usize = 500;

/// Validates that `key` is on the config allowlist.
pub fn validate_config_key(key: &str) -> Result<(), String> {
    if !ALLOWED_CONFIG_KEYS.contains(&key) {
        return Err(format!("Unknown config key: {key}"));
    }
    Ok(())
}

/// `validate_config_key` only checks that `key` is allowlisted; it says nothing
/// about whether `value` is something the app can actually use. A malformed
/// theme, language, currency code, or hidden-columns list would otherwise be
/// written to `app_config` and only surface as a confusing failure wherever
/// it's read back out.
pub fn validate_config_value(key: &str, value: &str) -> Result<(), String> {
    match key {
        "app_theme" => {
            if !["light", "dark", "system"].contains(&value) {
                return Err(format!(
                    "app_theme must be one of: light, dark, system (got: {value})"
                ));
            }
        }
        "app_language" => {
            if !SUPPORTED_LANGUAGES.contains(&value) {
                return Err(format!(
                    "app_language must be one of: {} (got: {value})",
                    SUPPORTED_LANGUAGES.join(", ")
                ));
            }
        }
        "app_color_scheme" => {
            if !SUPPORTED_COLOR_SCHEMES.contains(&value) {
                return Err(format!(
                    "app_color_scheme must be one of: {} (got: {value})",
                    SUPPORTED_COLOR_SCHEMES.join(", ")
                ));
            }
        }
        "base_currency" => {
            if value.len() != 3 || !value.bytes().all(|b| b.is_ascii_uppercase()) {
                return Err(format!(
                    "base_currency must be a 3-letter uppercase currency code (got: {value})"
                ));
            }
        }
        "holdings_hidden_columns" => {
            let columns: Vec<String> = serde_json::from_str(value).map_err(|_| {
                "holdings_hidden_columns must be a JSON array of column names".to_string()
            })?;
            if let Some(unknown) = columns
                .iter()
                .find(|c| !KNOWN_HOLDINGS_COLUMNS.contains(&c.as_str()))
            {
                return Err(format!(
                    "holdings_hidden_columns contains unknown column: {unknown}"
                ));
            }
        }
        "cost_basis_method" => {
            if !value.eq_ignore_ascii_case("avco") && !value.eq_ignore_ascii_case("fifo") {
                return Err(format!(
                    "cost_basis_method must be one of: avco, fifo (got: {value})"
                ));
            }
        }
        "notifications_enabled" | "auto_refresh_market_hours_only" => {
            if !["true", "false"].contains(&value) {
                return Err(format!("{key} must be one of: true, false (got: {value})"));
            }
        }
        _ => {
            if value.len() > MAX_CONFIG_VALUE_LEN {
                return Err(format!(
                    "{key} value exceeds max length of {MAX_CONFIG_VALUE_LEN} characters"
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
        // Regression guard for #644: get_config previously had no allowlist
        // at all, so any key — including internal/sensitive ones — could be
        // queried.
        let result = validate_config_key("some_internal_secret");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("some_internal_secret"));
    }

    #[test]
    fn validate_config_key_rejects_empty_key() {
        assert!(validate_config_key("").is_err());
    }

    #[test]
    fn validate_config_key_accepts_holdings_hidden_columns() {
        // Regression guard for #661/#662: the frontend persists which Holdings
        // table columns are hidden under this key, but it was missing from the
        // allowlist on both surfaces at different points in time.
        assert!(validate_config_key("holdings_hidden_columns").is_ok());
    }

    #[test]
    fn validate_config_value_accepts_known_theme_values() {
        for value in ["light", "dark", "system"] {
            assert!(validate_config_value("app_theme", value).is_ok());
        }
    }

    #[test]
    fn validate_config_value_rejects_unknown_theme() {
        // Regression guard for #690: previously any string was accepted for
        // app_theme regardless of whether the frontend could render it.
        assert!(validate_config_value("app_theme", "solarized").is_err());
        assert!(validate_config_value("app_theme", "").is_err());
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
        assert!(validate_config_value("app_language", "EN").is_err());
    }

    #[test]
    fn validate_config_value_accepts_known_color_schemes() {
        for value in ["default", "dracula", "synthwave", "nord", "warmLight"] {
            assert!(validate_config_value("app_color_scheme", value).is_ok());
        }
    }

    #[test]
    fn validate_config_value_rejects_unknown_color_scheme() {
        assert!(validate_config_value("app_color_scheme", "solarized").is_err());
        assert!(validate_config_value("app_color_scheme", "").is_err());
    }

    #[test]
    fn validate_config_value_accepts_valid_base_currency() {
        assert!(validate_config_value("base_currency", "CAD").is_ok());
        assert!(validate_config_value("base_currency", "USD").is_ok());
    }

    #[test]
    fn validate_config_value_rejects_malformed_base_currency() {
        assert!(validate_config_value("base_currency", "cad").is_err());
        assert!(validate_config_value("base_currency", "CA").is_err());
        assert!(validate_config_value("base_currency", "CAND").is_err());
        assert!(validate_config_value("base_currency", "C4D").is_err());
    }

    #[test]
    fn validate_config_value_accepts_valid_holdings_hidden_columns() {
        assert!(validate_config_value("holdings_hidden_columns", r#"["symbol","weight"]"#).is_ok());
        assert!(validate_config_value("holdings_hidden_columns", "[]").is_ok());
    }

    #[test]
    fn validate_config_value_rejects_non_json_holdings_hidden_columns() {
        assert!(validate_config_value("holdings_hidden_columns", "not json").is_err());
    }

    #[test]
    fn validate_config_value_rejects_unknown_column_in_holdings_hidden_columns() {
        // Regression guard for #690: an unknown column name would previously
        // be persisted verbatim and silently ignored by the frontend.
        assert!(validate_config_value("holdings_hidden_columns", r#"["notARealColumn"]"#).is_err());
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
        // to the generic max-length check, so any string was accepted and would
        // later fail to parse in compute_realized_gains.
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
        assert!(validate_config_value("cost_basis_method", "average").is_err());
    }

    #[test]
    fn validate_config_value_accepts_valid_boolean_flags() {
        for key in ["notifications_enabled", "auto_refresh_market_hours_only"] {
            assert!(validate_config_value(key, "true").is_ok());
            assert!(validate_config_value(key, "false").is_ok());
            assert!(validate_config_value(key, "yes").is_err());
        }
    }
}
