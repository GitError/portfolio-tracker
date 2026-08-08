//! Research watchlist field validation — name, currency, entry-price range,
//! and free-text research notes (thesis/catalysts/risks). Symbol *format*
//! validation (charset) is Yahoo-specific and lives in `src-tauri::price`
//! since it's shared with the rest of the price-fetching pipeline.

/// Max length for a watchlist's display name.
pub const MAX_WATCHLIST_NAME_LEN: usize = 100;

/// Max length for each of a watchlist item's free-text research fields
/// (thesis, catalysts, risks).
pub const MAX_WATCHLIST_NOTE_LEN: usize = 4000;

/// Validates a watchlist's display name: non-empty (after trimming) and
/// within `MAX_WATCHLIST_NAME_LEN`.
pub fn validate_watchlist_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("name must not be empty".to_string());
    }
    if trimmed.chars().count() > MAX_WATCHLIST_NAME_LEN {
        return Err(format!(
            "name must be at most {MAX_WATCHLIST_NAME_LEN} characters"
        ));
    }
    Ok(())
}

/// Validates a watchlist item's currency code (2-3 uppercase letters),
/// mirroring `validate_alert_currency`.
pub fn validate_watchlist_item_currency(currency: &str) -> Result<(), String> {
    let currency = currency.trim();
    if !(2..=3).contains(&currency.len()) || !currency.chars().all(|c| c.is_ascii_uppercase()) {
        return Err("currency must be 2-3 uppercase letters".to_string());
    }
    Ok(())
}

/// Validates one of a watchlist item's free-text research fields against
/// `MAX_WATCHLIST_NOTE_LEN`. Empty/absent values are always valid — these
/// fields are optional research notes, not required input.
pub fn validate_watchlist_note(field_name: &str, value: &str) -> Result<(), String> {
    if value.chars().count() > MAX_WATCHLIST_NOTE_LEN {
        return Err(format!(
            "{field_name} must be at most {MAX_WATCHLIST_NOTE_LEN} characters"
        ));
    }
    Ok(())
}

/// Validates a watchlist item's entry-price range: each bound (if present)
/// must be finite and non-negative, and low must not exceed high.
pub fn validate_entry_price_range(low: Option<f64>, high: Option<f64>) -> Result<(), String> {
    if let Some(low) = low {
        if !low.is_finite() || low < 0.0 {
            return Err("entry_price_low must be a non-negative finite number".to_string());
        }
    }
    if let Some(high) = high {
        if !high.is_finite() || high < 0.0 {
            return Err("entry_price_high must be a non-negative finite number".to_string());
        }
    }
    if let (Some(low), Some(high)) = (low, high) {
        if low > high {
            return Err("entry_price_low must not exceed entry_price_high".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_watchlist_name_rejects_empty_and_whitespace() {
        assert!(validate_watchlist_name("").is_err());
        assert!(validate_watchlist_name("   ").is_err());
    }

    #[test]
    fn validate_watchlist_name_rejects_over_max_length() {
        let name = "a".repeat(MAX_WATCHLIST_NAME_LEN + 1);
        assert!(validate_watchlist_name(&name).is_err());
    }

    #[test]
    fn validate_watchlist_name_accepts_within_max_length() {
        assert!(validate_watchlist_name("Growth Ideas").is_ok());
        assert!(validate_watchlist_name(&"a".repeat(MAX_WATCHLIST_NAME_LEN)).is_ok());
    }

    #[test]
    fn validate_watchlist_item_currency_rejects_malformed() {
        assert!(validate_watchlist_item_currency("").is_err());
        assert!(validate_watchlist_item_currency("A").is_err());
        assert!(validate_watchlist_item_currency("ABCD").is_err());
        assert!(validate_watchlist_item_currency("usd").is_err());
    }

    #[test]
    fn validate_watchlist_item_currency_accepts_valid() {
        assert!(validate_watchlist_item_currency("USD").is_ok());
        assert!(validate_watchlist_item_currency("CAD").is_ok());
    }

    #[test]
    fn validate_watchlist_note_accepts_empty_and_within_max() {
        assert!(validate_watchlist_note("thesis", "").is_ok());
        assert!(validate_watchlist_note("thesis", &"a".repeat(MAX_WATCHLIST_NOTE_LEN)).is_ok());
    }

    #[test]
    fn validate_watchlist_note_rejects_over_max_length() {
        let note = "a".repeat(MAX_WATCHLIST_NOTE_LEN + 1);
        let err = validate_watchlist_note("thesis", &note).unwrap_err();
        assert!(err.contains("thesis"));
    }

    #[test]
    fn validate_entry_price_range_accepts_none_and_valid_bounds() {
        assert!(validate_entry_price_range(None, None).is_ok());
        assert!(validate_entry_price_range(Some(10.0), None).is_ok());
        assert!(validate_entry_price_range(None, Some(20.0)).is_ok());
        assert!(validate_entry_price_range(Some(10.0), Some(20.0)).is_ok());
        assert!(validate_entry_price_range(Some(10.0), Some(10.0)).is_ok());
    }

    #[test]
    fn validate_entry_price_range_rejects_low_above_high() {
        assert!(validate_entry_price_range(Some(20.0), Some(10.0)).is_err());
    }

    #[test]
    fn validate_entry_price_range_rejects_negative_or_non_finite() {
        assert!(validate_entry_price_range(Some(-1.0), None).is_err());
        assert!(validate_entry_price_range(None, Some(f64::NAN)).is_err());
        assert!(validate_entry_price_range(Some(f64::INFINITY), None).is_err());
    }
}
