//! Holding field validation — quantity/cost basis/currency, target weight,
//! and the dividend/maturity fields attached to a holding.

/// Dividend frequencies accepted by the CSV import layer (`src-tauri/src/csv.rs`),
/// mirrored here so the MCP `add_holding` tool applies the same set.
pub const VALID_DIVIDEND_FREQUENCIES: &[&str] =
    &["monthly", "quarterly", "semi-annual", "annual", "irregular"];

/// Tolerance applied to target-weight-sum comparisons against 100%, to absorb
/// floating-point rounding without letting a materially-over-budget value through.
pub const WEIGHT_EPSILON: f64 = 0.001;

/// Validates a holding's quantity, cost basis, and currency. Returns the
/// normalized (uppercase, trimmed) currency code on success.
pub fn validate_holding_fields(
    quantity: f64,
    cost_basis: f64,
    currency: &str,
) -> Result<String, String> {
    if quantity <= 0.0 || !quantity.is_finite() {
        return Err("quantity must be a positive finite number".to_string());
    }
    if cost_basis < 0.0 || !cost_basis.is_finite() {
        return Err("costBasis must be a non-negative finite number".to_string());
    }
    let currency = currency.trim().to_uppercase();
    if currency.len() != 3 || !currency.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err("currency must be a 3-letter ISO currency code".to_string());
    }
    Ok(currency)
}

/// Validates an optional target weight: when present, must be a finite
/// percentage in [0, 100]. A negative or out-of-range value would otherwise
/// silently persist and produce nonsensical rebalance suggestions.
pub fn validate_target_weight(target_weight: Option<f64>) -> Result<(), String> {
    if let Some(weight) = target_weight {
        if !weight.is_finite() || !(0.0..=100.0).contains(&weight) {
            return Err("targetWeight must be a finite number between 0 and 100".to_string());
        }
    }
    Ok(())
}

/// Returns true when adding/updating a holding with `new_weight` would push the
/// portfolio's total target weight (across all other holdings, summing to
/// `existing_sum`) over 100%. Non-positive weights never consume budget.
pub fn exceeds_target_weight_budget(new_weight: f64, existing_sum: f64) -> bool {
    new_weight > 0.0 && existing_sum + new_weight > 100.0 + WEIGHT_EPSILON
}

/// Validates the optional dividend/maturity fields shared by `add_holding`/`update_holding`.
/// Mirrors the checks the CSV import layer applies in `src-tauri/src/csv.rs`.
pub fn validate_holding_dividend_fields(
    indicated_annual_dividend: Option<f64>,
    dividend_frequency: Option<&str>,
    maturity_date: Option<&str>,
) -> Result<(), String> {
    if let Some(amount) = indicated_annual_dividend {
        if !amount.is_finite() || amount < 0.0 {
            return Err("indicatedAnnualDividend must be a non-negative finite number".to_string());
        }
    }
    if let Some(freq) = dividend_frequency {
        let normalized = freq.trim().to_lowercase();
        if !VALID_DIVIDEND_FREQUENCIES.contains(&normalized.as_str()) {
            return Err(format!(
                "dividendFrequency must be one of: {}",
                VALID_DIVIDEND_FREQUENCIES.join(", ")
            ));
        }
    }
    if let Some(date) = maturity_date {
        if chrono::NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d").is_err() {
            return Err("maturityDate must be a valid ISO date (YYYY-MM-DD)".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_holding_fields_rejects_nan_quantity() {
        assert!(validate_holding_fields(f64::NAN, 100.0, "USD").is_err());
    }

    #[test]
    fn validate_holding_fields_rejects_infinite_cost_basis() {
        assert!(validate_holding_fields(1.0, f64::INFINITY, "USD").is_err());
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
    fn validate_target_weight_rejects_out_of_range_and_nan() {
        assert!(validate_target_weight(Some(-1.0)).is_err());
        assert!(validate_target_weight(Some(100.001)).is_err());
        assert!(validate_target_weight(Some(f64::NAN)).is_err());
        assert!(validate_target_weight(Some(f64::INFINITY)).is_err());
        assert!(validate_target_weight(Some(50.0)).is_ok());
        assert!(validate_target_weight(Some(0.0)).is_ok());
        assert!(validate_target_weight(Some(100.0)).is_ok());
        assert!(validate_target_weight(None).is_ok());
    }

    #[test]
    fn exceeds_target_weight_budget_rejects_when_total_exceeds_100() {
        assert!(exceeds_target_weight_budget(50.0, 60.0));
        assert!(!exceeds_target_weight_budget(40.0, 60.0));
        // Non-positive weights don't consume budget and are always allowed.
        assert!(!exceeds_target_weight_budget(0.0, 99.0));
        assert!(!exceeds_target_weight_budget(-5.0, 99.0));
    }

    #[test]
    fn exceeds_target_weight_budget_allows_exactly_100() {
        assert!(!exceeds_target_weight_budget(40.0, 60.0));
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
}
