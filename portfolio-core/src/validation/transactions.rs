//! Buy/sell transaction field validation.

/// Validates transaction quantity/price for `add_transaction`.
pub fn validate_transaction_fields(quantity: f64, price: f64) -> Result<(), String> {
    if quantity <= 0.0 || !quantity.is_finite() {
        return Err("Transaction quantity must be a positive finite number".to_string());
    }
    if price < 0.0 || !price.is_finite() {
        return Err("Transaction price must be a non-negative finite number".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(validate_transaction_fields(5.0, f64::NAN).is_err());
    }

    #[test]
    fn validate_transaction_fields_accepts_valid_input() {
        assert!(validate_transaction_fields(5.0, 0.0).is_ok());
        assert!(validate_transaction_fields(5.0, 120.5).is_ok());
    }
}
