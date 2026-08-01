use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::db;
use crate::validation;

use super::PortfolioMcpServer;

// ── Params ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetConfigParams {
    /// Configuration key.  Known keys: base_currency, cost_basis_method,
    /// auto_refresh_interval_ms, auto_refresh_market_hours_only, app_theme,
    /// app_language.
    pub key: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetConfigParams {
    /// Configuration key (see GetConfigParams for known keys).
    pub key: String,
    /// New value string.
    pub value: String,
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ConfigValue {
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SetConfigResult {
    pub key: String,
    pub value: String,
    pub ok: bool,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn get_config(
    pool: &SqlitePool,
    params: GetConfigParams,
) -> Result<ConfigValue, McpError> {
    validation::validate_config_key(&params.key)?;
    let value = db::get_config(pool, &params.key)
        .await
        .map_err(PortfolioMcpServer::tool_error)?;

    Ok(ConfigValue {
        key: params.key,
        value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("open in-memory sqlite");
        sqlx::query("CREATE TABLE app_config (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
            .execute(&pool)
            .await
            .expect("create app_config table");
        pool
    }

    #[tokio::test]
    async fn get_config_rejects_unknown_key() {
        // Regression guard for #662: get_config previously had no allowlist
        // check at all (unlike set_config), so any key — including
        // internal/sensitive ones — could be read.
        let pool = test_pool().await;
        let result = get_config(
            &pool,
            GetConfigParams {
                key: "some_internal_secret".to_string(),
            },
        )
        .await;
        assert!(result.is_err(), "unknown config key must be rejected");
    }

    #[tokio::test]
    async fn get_config_accepts_allowed_key() {
        let pool = test_pool().await;
        let result = get_config(
            &pool,
            GetConfigParams {
                key: "base_currency".to_string(),
            },
        )
        .await;
        assert!(result.is_ok());
    }
}

pub async fn set_config(
    pool: &SqlitePool,
    params: SetConfigParams,
) -> Result<SetConfigResult, McpError> {
    validation::validate_config_key(&params.key)?;
    let value = if params.key == "cost_basis_method" {
        params.value.to_lowercase()
    } else {
        params.value
    };
    validation::validate_config_value(&params.key, &value)?;

    db::set_config(pool, &params.key, &value)
        .await
        .map_err(PortfolioMcpServer::tool_error)?;

    Ok(SetConfigResult {
        key: params.key,
        value,
        ok: true,
    })
}
