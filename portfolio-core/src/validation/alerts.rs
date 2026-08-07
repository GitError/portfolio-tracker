//! Price alert field validation — threshold, currency, and free-text note.

/// Max length for a price alert's free-text note, to prevent abuse.
pub const MAX_ALERT_NOTE_LEN: usize = 500;

/// Validates a price alert's threshold.
pub fn validate_alert_threshold(threshold: f64) -> Result<(), String> {
    if !threshold.is_finite() || threshold <= 0.0 {
        return Err("threshold must be a positive finite number".to_string());
    }
    Ok(())
}

/// Validates a price alert's currency code (2-3 uppercase letters).
pub fn validate_alert_currency(currency: &str) -> Result<(), String> {
    let currency = currency.trim();
    if !(2..=3).contains(&currency.len()) || !currency.chars().all(|c| c.is_ascii_uppercase()) {
        return Err("currency must be 2-3 uppercase letters".to_string());
    }
    Ok(())
}

/// Validates a price alert's free-text note against `MAX_ALERT_NOTE_LEN`.
pub fn validate_alert_note(note: &str) -> Result<(), String> {
    if note.chars().count() > MAX_ALERT_NOTE_LEN {
        return Err(format!(
            "note must be at most {MAX_ALERT_NOTE_LEN} characters"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
