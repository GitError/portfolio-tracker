//! Dividend record field validation — amount and ex/pay date ordering.

/// Validates dividend input fields shared by `add_dividend`.
pub fn validate_dividend_fields(
    amount_per_unit: f64,
    ex_date: &str,
    pay_date: &str,
) -> Result<(), String> {
    if !amount_per_unit.is_finite() || amount_per_unit <= 0.0 {
        return Err("amountPerUnit must be a finite number greater than 0".to_string());
    }
    if chrono::NaiveDate::parse_from_str(ex_date.trim(), "%Y-%m-%d").is_err() {
        return Err("exDate must be a valid ISO date (YYYY-MM-DD)".to_string());
    }
    if chrono::NaiveDate::parse_from_str(pay_date.trim(), "%Y-%m-%d").is_err() {
        return Err("payDate must be a valid ISO date (YYYY-MM-DD)".to_string());
    }
    if pay_date < ex_date {
        return Err("payDate must not be before exDate".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(validate_dividend_fields(1.0, "not-a-date", "2024-01-15").is_err());
        assert!(validate_dividend_fields(1.0, "2024-01-01", "").is_err());
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
