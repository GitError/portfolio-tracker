//! Named account field validation.

/// Account types accepted by `create_account`/`add_account`/`update_account`.
pub const VALID_ACCOUNT_TYPES: &[&str] =
    &["tfsa", "rrsp", "fhsa", "taxable", "crypto", "cash", "other"];

/// Validates an account's name and type, shared by `add_account`/`update_account`.
/// Returns the trimmed name on success.
pub fn validate_account_fields(name: &str, account_type: &str) -> Result<String, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Account name cannot be empty".to_string());
    }
    if !VALID_ACCOUNT_TYPES.contains(&account_type) {
        return Err(format!("Invalid account type: {account_type}"));
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn validate_account_fields_accepts_every_valid_type() {
        for account_type in VALID_ACCOUNT_TYPES {
            assert!(
                validate_account_fields("Account", account_type).is_ok(),
                "{account_type} should be accepted"
            );
        }
    }
}
