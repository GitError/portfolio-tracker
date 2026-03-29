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
            sqlx::Error::RowNotFound => AppError::NotFound(e.to_string()),
            _ => AppError::Database(e.to_string()),
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
        AppError::Validation(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Validation(s.to_string())
    }
}
