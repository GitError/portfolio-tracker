pub mod alerts;
pub mod config;
pub mod holdings;
pub mod portfolio;
pub mod stress;
pub mod transactions;

use rmcp::{
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    tool, Error as McpError, ServerHandler,
};
use sqlx::SqlitePool;

/// The MCP server that exposes portfolio tools.
#[derive(Clone)]
pub struct PortfolioMcpServer {
    pool: SqlitePool,
}

impl PortfolioMcpServer {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Serialise a value to JSON text content for a `CallToolResult`.
    pub(crate) fn json_content<T: serde::Serialize>(value: &T) -> Result<CallToolResult, McpError> {
        let text = serde_json::to_string_pretty(value)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    /// Wrap an anyhow error as an MCP tool error.
    pub(crate) fn tool_error(err: anyhow::Error) -> McpError {
        McpError::internal_error(err.to_string(), None)
    }
}

#[tool(tool_box)]
impl PortfolioMcpServer {
    // ── Holdings ──────────────────────────────────────────────────────────────

    #[tool(description = "List all current holdings in the portfolio (excluding deleted).")]
    async fn list_holdings(&self) -> Result<CallToolResult, McpError> {
        holdings::list_holdings(&self.pool)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    #[tool(description = "Add a new holding to the portfolio.")]
    async fn add_holding(
        &self,
        #[tool(aggr)] input: holdings::AddHoldingParams,
    ) -> Result<CallToolResult, McpError> {
        holdings::add_holding(&self.pool, input)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    #[tool(description = "Soft-delete a holding by its UUID.")]
    async fn delete_holding(
        &self,
        #[tool(aggr)] input: holdings::DeleteHoldingParams,
    ) -> Result<CallToolResult, McpError> {
        holdings::delete_holding(&self.pool, input)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    // ── Transactions ──────────────────────────────────────────────────────────

    #[tool(description = "List all buy/sell transactions across all holdings.")]
    async fn list_transactions(&self) -> Result<CallToolResult, McpError> {
        transactions::list_transactions(&self.pool)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    #[tool(description = "Record a new buy or sell transaction for a holding.")]
    async fn add_transaction(
        &self,
        #[tool(aggr)] input: transactions::AddTransactionParams,
    ) -> Result<CallToolResult, McpError> {
        transactions::add_transaction(&self.pool, input)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    #[tool(description = "Soft-delete a transaction by its UUID.")]
    async fn delete_transaction(
        &self,
        #[tool(aggr)] input: transactions::DeleteTransactionParams,
    ) -> Result<CallToolResult, McpError> {
        transactions::delete_transaction(&self.pool, input)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    // ── Alerts ────────────────────────────────────────────────────────────────

    #[tool(description = "List all price alerts.")]
    async fn list_alerts(&self) -> Result<CallToolResult, McpError> {
        alerts::list_alerts(&self.pool)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    #[tool(description = "Create a new price alert for a symbol.")]
    async fn add_alert(
        &self,
        #[tool(aggr)] input: alerts::AddAlertParams,
    ) -> Result<CallToolResult, McpError> {
        alerts::add_alert(&self.pool, input)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    #[tool(description = "Delete a price alert by its UUID.")]
    async fn delete_alert(
        &self,
        #[tool(aggr)] input: alerts::DeleteAlertParams,
    ) -> Result<CallToolResult, McpError> {
        alerts::delete_alert(&self.pool, input)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    #[tool(description = "Reset a triggered price alert so it can fire again.")]
    async fn reset_alert(
        &self,
        #[tool(aggr)] input: alerts::ResetAlertParams,
    ) -> Result<CallToolResult, McpError> {
        alerts::reset_alert(&self.pool, input)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    // ── Portfolio snapshot ─────────────────────────────────────────────────────

    #[tool(
        description = "Get the current portfolio snapshot: all holdings with cached prices, \
                        market values, gain/loss, weights, and aggregate totals. Values are \
                        expressed in the configured base currency (default CAD)."
    )]
    async fn get_portfolio_snapshot(&self) -> Result<CallToolResult, McpError> {
        portfolio::get_portfolio_snapshot(&self.pool)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    // ── Stress test ───────────────────────────────────────────────────────────

    #[tool(
        description = "Run a stress-test scenario against the current portfolio. Supply a scenario \
                        name and a map of asset-class/FX shocks (e.g. {\"stock\": -0.2, \
                        \"fx_usd_cad\": 0.05}). Keys: stock, etf, crypto, cash, \
                        fx_<from>_<to> (lower-cased currency codes)."
    )]
    async fn run_stress_test(
        &self,
        #[tool(aggr)] input: stress::StressTestParams,
    ) -> Result<CallToolResult, McpError> {
        stress::run_stress_test(&self.pool, input)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    // ── Config ────────────────────────────────────────────────────────────────

    #[tool(
        description = "Read a configuration value by key (e.g. base_currency, \
                        auto_refresh_interval_ms, cost_basis_method)."
    )]
    async fn get_config(
        &self,
        #[tool(aggr)] input: config::GetConfigParams,
    ) -> Result<CallToolResult, McpError> {
        config::get_config(&self.pool, input)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    #[tool(
        description = "Write a configuration value. Known keys: base_currency, \
                        cost_basis_method, auto_refresh_interval_ms, \
                        auto_refresh_market_hours_only, app_theme, app_language."
    )]
    async fn set_config(
        &self,
        #[tool(aggr)] input: config::SetConfigParams,
    ) -> Result<CallToolResult, McpError> {
        config::set_config(&self.pool, input)
            .await
            .and_then(|v| Self::json_content(&v))
    }
}

#[tool(tool_box)]
impl ServerHandler for PortfolioMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "portfolio-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(
                "Portfolio Tracker MCP server. Use list_holdings and get_portfolio_snapshot to \
                 read the current portfolio state. Use add_holding / delete_holding to manage \
                 positions. Use run_stress_test to simulate market scenarios."
                    .to_string(),
            ),
        }
    }
}
