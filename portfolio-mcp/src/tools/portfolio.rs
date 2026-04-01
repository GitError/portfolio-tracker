use chrono::Utc;
use rmcp::Error as McpError;
use sqlx::SqlitePool;

use crate::{db, snapshot, types::PortfolioSnapshot};

use super::PortfolioMcpServer;

pub async fn get_portfolio_snapshot(pool: &SqlitePool) -> Result<PortfolioSnapshot, McpError> {
    // Resolve base currency from config (default: CAD).
    let base_currency = db::get_config(pool, "base_currency")
        .await
        .map_err(PortfolioMcpServer::tool_error)?
        .unwrap_or_else(|| "CAD".to_string());

    let holdings = db::get_all_holdings(pool)
        .await
        .map_err(PortfolioMcpServer::tool_error)?;

    let cached_prices = db::get_cached_prices(pool)
        .await
        .map_err(PortfolioMcpServer::tool_error)?;

    let cached_fx = db::get_fx_rates(pool)
        .await
        .map_err(PortfolioMcpServer::tool_error)?;

    let last_updated = Utc::now().to_rfc3339();

    // `realized_gains` and `annual_dividend_income` are not computed here to
    // keep this read path lightweight.  They default to 0 in the MCP context;
    // the Tauri app performs the full calculation via separate DB queries.
    let snapshot = snapshot::build_portfolio_snapshot(
        &holdings,
        &cached_prices,
        &cached_fx,
        &base_currency,
        last_updated,
        0.0,
        0.0,
    );

    Ok(snapshot)
}
