use serde::Serialize;

/// Typed application error returned by all Tauri commands.
///
/// Serializes as `{ "type": "validation", "message": "..." }` so the
/// frontend can `switch(error.type)` for targeted UI messages.
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "message", rename_all = "snake_case")]
pub enum AppError {
    Validation(String),
    Database(String),
    Network(String),
    NotFound(String),
    Conflict(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Validation(m) => write!(f, "Validation error: {m}"),
            AppError::Database(m) => write!(f, "Database error: {m}"),
            AppError::Network(m) => write!(f, "Network error: {m}"),
            AppError::NotFound(m) => write!(f, "Not found: {m}"),
            AppError::Conflict(m) => write!(f, "Conflict: {m}"),
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => AppError::NotFound("Record not found".to_string()),
            _ => {
                // Log the full error internally but return a sanitized message to avoid
                // leaking internal schema details to the frontend.
                tracing::error!("Database error: {}", e);
                AppError::Database("A database error occurred".to_string())
            }
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Network(e.to_string())
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        // Most `db.rs` functions return `Result<T, String>` (built via
        // `.map_err(|e| e.to_string())`), so by the time an error reaches
        // this conversion the original `sqlx::Error` variant is gone.
        // Pattern-match the well-known SQLite error text so uniqueness/FK
        // violations and missing-row lookups still surface as Conflict/
        // NotFound to the frontend instead of a blanket Validation error.
        if s.contains("UNIQUE constraint failed") || s.contains("FOREIGN KEY constraint failed") {
            AppError::Conflict(s)
        } else if s.contains("no rows returned by a query") {
            AppError::NotFound(s)
        } else {
            AppError::Validation(s)
        }
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::from(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_string_classifies_unique_constraint_as_conflict() {
        let err = AppError::from(
            "error returned from database: (code: 2067) UNIQUE constraint failed: holdings.id"
                .to_string(),
        );
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[test]
    fn from_string_classifies_foreign_key_violation_as_conflict() {
        let err = AppError::from(
            "error returned from database: (code: 787) FOREIGN KEY constraint failed".to_string(),
        );
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[test]
    fn from_string_classifies_row_not_found_as_not_found() {
        let err = AppError::from(
            "no rows returned by a query that expected to return at least one row".to_string(),
        );
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn from_string_falls_back_to_validation_for_unrecognized_errors() {
        let err = AppError::from("quantity must be positive".to_string());
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn from_str_uses_same_classification_as_from_string() {
        let err = AppError::from("UNIQUE constraint failed: accounts.id");
        assert!(matches!(err, AppError::Conflict(_)));
    }
}
