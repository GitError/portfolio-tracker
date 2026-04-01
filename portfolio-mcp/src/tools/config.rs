use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::db;

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
    let value = db::get_config(pool, &params.key)
        .await
        .map_err(PortfolioMcpServer::tool_error)?;

    Ok(ConfigValue {
        key: params.key,
        value,
    })
}

pub async fn set_config(
    pool: &SqlitePool,
    params: SetConfigParams,
) -> Result<SetConfigResult, McpError> {
    db::set_config(pool, &params.key, &params.value)
        .await
        .map_err(PortfolioMcpServer::tool_error)?;

    Ok(SetConfigResult {
        key: params.key,
        value: params.value,
        ok: true,
    })
}
