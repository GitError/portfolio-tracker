use rmcp::Error as McpError;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    db,
    types::{AccountType, AssetType, Holding, HoldingId, HoldingInput},
};

use super::PortfolioMcpServer;

// ── Params ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddHoldingParams {
    /// Ticker symbol (e.g. "AAPL", "BTC-USD", "CASH-CAD").
    pub symbol: String,
    /// Human-readable name.
    pub name: String,
    /// Asset class: "stock", "etf", "crypto", or "cash".
    pub asset_type: String,
    /// Account type: "tfsa", "rrsp", "fhsa", "taxable", "crypto", "cash", or "other".
    pub account: String,
    /// Optional explicit account UUID (overrides account-type lookup).
    pub account_id: Option<String>,
    /// Number of units held.
    pub quantity: f64,
    /// Average cost per unit in the holding's native currency.
    pub cost_basis: f64,
    /// ISO 4217 currency code (e.g. "CAD", "USD").
    pub currency: String,
    /// Exchange identifier (e.g. "TSX", "NASDAQ").
    pub exchange: String,
    /// Target portfolio weight as a percentage (0–100).
    pub target_weight: f64,
    /// Indicated annual dividend per unit in the dividend currency.
    pub indicated_annual_dividend: Option<f64>,
    pub indicated_annual_dividend_currency: Option<String>,
    /// Dividend frequency: "monthly", "quarterly", "semi-annual", "annual", "irregular".
    pub dividend_frequency: Option<String>,
    /// Maturity date for fixed-income positions (ISO 8601 date string).
    pub maturity_date: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteHoldingParams {
    /// UUID of the holding to delete.
    pub id: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list_holdings(pool: &SqlitePool) -> Result<Vec<Holding>, McpError> {
    db::get_all_holdings(pool)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn add_holding(pool: &SqlitePool, params: AddHoldingParams) -> Result<Holding, McpError> {
    let asset_type = params
        .asset_type
        .parse::<AssetType>()
        .map_err(|e| McpError::invalid_params(e, None))?;

    let account = params
        .account
        .parse::<AccountType>()
        .map_err(|e| McpError::invalid_params(e, None))?;

    let input = HoldingInput {
        symbol: params.symbol,
        name: params.name,
        asset_type,
        account,
        account_id: params.account_id,
        quantity: params.quantity,
        cost_basis: params.cost_basis,
        currency: params.currency,
        exchange: params.exchange,
        target_weight: params.target_weight,
        indicated_annual_dividend: params.indicated_annual_dividend,
        indicated_annual_dividend_currency: params.indicated_annual_dividend_currency,
        dividend_frequency: params.dividend_frequency,
        maturity_date: params.maturity_date,
    };

    db::insert_holding(pool, input)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn delete_holding(
    pool: &SqlitePool,
    params: DeleteHoldingParams,
) -> Result<bool, McpError> {
    let id = HoldingId(params.id);
    db::delete_holding(pool, &id)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}
