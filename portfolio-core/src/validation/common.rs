//! Validation rules with no domain affiliation — shared by holdings,
//! transactions, alerts, dividends, and accounts.

/// Rejects a blank/whitespace-only required string field.
pub fn validate_non_empty(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    Ok(())
}

/// Validates that an ID string is non-empty and a syntactically valid UUID.
/// All IDs in this app are UUID v4 strings generated via `uuid::Uuid::new_v4()`;
/// a malformed ID would otherwise silently no-op in SQLite (0 rows affected)
/// instead of surfacing a clear error (see #685).
pub fn validate_id(field: &str, id: &str) -> Result<(), String> {
    if id.trim().is_empty() || uuid::Uuid::parse_str(id.trim()).is_err() {
        return Err(format!("Invalid {field}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_non_empty_rejects_blank_and_whitespace() {
        assert!(validate_non_empty("symbol", "").is_err());
        assert!(validate_non_empty("symbol", "   ").is_err());
        assert!(validate_non_empty("symbol", "AAPL").is_ok());
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
}
