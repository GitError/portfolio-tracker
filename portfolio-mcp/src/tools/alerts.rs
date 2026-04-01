use rmcp::Error as McpError;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    db,
    types::{AlertDirection, AlertId, PriceAlert, PriceAlertInput},
};

use super::PortfolioMcpServer;

// ── Params ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddAlertParams {
    /// Ticker symbol to watch (e.g. "AAPL").
    pub symbol: String,
    /// Alert direction: "above" fires when price rises above threshold,
    /// "below" fires when price drops below threshold.
    pub direction: String,
    /// Price threshold in the specified currency.
    pub threshold: f64,
    /// ISO 4217 currency code for the threshold (e.g. "USD").
    pub currency: String,
    /// Optional free-text note for this alert.
    pub note: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteAlertParams {
    /// UUID of the alert to delete.
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ResetAlertParams {
    /// UUID of the triggered alert to reset.
    pub id: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list_alerts(pool: &SqlitePool) -> Result<Vec<PriceAlert>, McpError> {
    db::get_alerts(pool)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn add_alert(pool: &SqlitePool, params: AddAlertParams) -> Result<PriceAlert, McpError> {
    let direction = params
        .direction
        .parse::<AlertDirection>()
        .map_err(|e| McpError::invalid_params(e, None))?;

    let input = PriceAlertInput {
        symbol: params.symbol,
        direction,
        threshold: params.threshold,
        currency: params.currency,
        note: params.note,
    };

    db::insert_alert(pool, input)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn delete_alert(pool: &SqlitePool, params: DeleteAlertParams) -> Result<bool, McpError> {
    let id = AlertId(params.id);
    db::delete_alert(pool, &id)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn reset_alert(pool: &SqlitePool, params: ResetAlertParams) -> Result<bool, McpError> {
    let id = AlertId(params.id);
    db::reset_alert(pool, &id)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}
