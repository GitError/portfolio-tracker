pub mod accounts;
pub mod alerts;
pub mod config;
pub mod dividends;
pub mod holdings;
pub mod portfolio;
pub mod stress;
pub mod transactions;

use rmcp::{
    handler::server::tool::ToolCallContext,
    model::{
        CallToolRequestParam, CallToolResult, Content, Implementation, ListToolsResult,
        PaginatedRequestParam, ProtocolVersion, ServerCapabilities, ServerInfo, Tool,
    },
    service::{RequestContext, RoleServer},
    tool, Error as McpError, ServerHandler,
};
use sqlx::SqlitePool;

/// Which mutating tool categories are registered on this server instance.
///
/// Both flags default to `false` (read-only) unless explicitly opted into via
/// the `PORTFOLIO_MCP_WRITE_ENABLED` / `PORTFOLIO_MCP_DESTRUCTIVE_WRITE_ENABLED`
/// environment variables (see `main.rs`).
#[derive(Debug, Clone, Copy, Default)]
pub struct McpAccess {
    pub write_enabled: bool,
    pub destructive_enabled: bool,
}

impl McpAccess {
    fn permits(&self, category: ToolCategory) -> bool {
        match category {
            ToolCategory::Read => true,
            ToolCategory::Write => self.write_enabled,
            ToolCategory::Destructive => self.destructive_enabled,
        }
    }

    pub fn mode_label(&self) -> &'static str {
        match (self.write_enabled, self.destructive_enabled) {
            (false, false) => "read-only",
            (true, false) => "write-enabled (non-destructive)",
            (false, true) => "destructive-only (unusual: write tools still disabled)",
            (true, true) => "write-enabled (including destructive operations)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolCategory {
    Read,
    Write,
    Destructive,
}

impl ToolCategory {
    fn enable_hint(self) -> &'static str {
        match self {
            ToolCategory::Read => "",
            ToolCategory::Write => "set PORTFOLIO_MCP_WRITE_ENABLED=true to enable it",
            ToolCategory::Destructive => {
                "set PORTFOLIO_MCP_DESTRUCTIVE_WRITE_ENABLED=true to enable it"
            }
        }
    }
}

/// Classify a tool by name so it can be gated by [`McpAccess`].
///
/// New mutating tools MUST be added here (as `Write` or `Destructive`) —
/// anything not listed defaults to `Read` and is always registered, which
/// would silently bypass the opt-in write gate.
fn tool_category(name: &str) -> ToolCategory {
    match name {
        "delete_holding" | "delete_dividend" | "delete_transaction" | "delete_alert" => {
            ToolCategory::Destructive
        }
        "add_holding" | "update_holding" | "create_account" | "add_dividend"
        | "add_transaction" | "add_alert" | "reset_alert" | "set_config" => ToolCategory::Write,
        _ => ToolCategory::Read,
    }
}

/// The MCP server that exposes portfolio tools.
#[derive(Clone)]
pub struct PortfolioMcpServer {
    pool: SqlitePool,
    access: McpAccess,
}

impl PortfolioMcpServer {
    pub fn new(pool: SqlitePool, access: McpAccess) -> Self {
        Self { pool, access }
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

    /// Tools visible given this server's current access mode.
    fn visible_tools(&self) -> Vec<Tool> {
        Self::tool_box()
            .list()
            .into_iter()
            .filter(|t| self.access.permits(tool_category(t.name.as_ref())))
            .collect()
    }

    /// Reject a tool call whose category isn't enabled for this server instance.
    fn ensure_tool_permitted(&self, name: &str) -> Result<(), McpError> {
        let category = tool_category(name);
        if self.access.permits(category) {
            return Ok(());
        }
        Err(McpError::invalid_request(
            format!(
                "Tool '{name}' is disabled in the current mode ({}); {}",
                self.access.mode_label(),
                category.enable_hint(),
            ),
            None,
        ))
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

    #[tool(description = "Update an existing holding's editable fields (full replace).")]
    async fn update_holding(
        &self,
        #[tool(aggr)] input: holdings::UpdateHoldingParams,
    ) -> Result<CallToolResult, McpError> {
        holdings::update_holding(&self.pool, input)
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

    // ── Accounts ──────────────────────────────────────────────────────────────

    #[tool(description = "List all accounts (id, name, type, institution).")]
    async fn list_accounts(&self) -> Result<CallToolResult, McpError> {
        accounts::list_accounts(&self.pool)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    #[tool(description = "Create a new account.")]
    async fn create_account(
        &self,
        #[tool(aggr)] input: accounts::CreateAccountParams,
    ) -> Result<CallToolResult, McpError> {
        accounts::create_account(&self.pool, input)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    // ── Dividends ─────────────────────────────────────────────────────────────

    #[tool(description = "List dividend records, optionally filtered by holding UUID.")]
    async fn list_dividends(
        &self,
        #[tool(aggr)] input: dividends::ListDividendsParams,
    ) -> Result<CallToolResult, McpError> {
        dividends::list_dividends(&self.pool, input)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    #[tool(description = "Record a new dividend payment for a holding.")]
    async fn add_dividend(
        &self,
        #[tool(aggr)] input: dividends::AddDividendParams,
    ) -> Result<CallToolResult, McpError> {
        dividends::add_dividend(&self.pool, input)
            .await
            .and_then(|v| Self::json_content(&v))
    }

    #[tool(description = "Soft-delete a dividend record by its UUID.")]
    async fn delete_dividend(
        &self,
        #[tool(aggr)] input: dividends::DeleteDividendParams,
    ) -> Result<CallToolResult, McpError> {
        dividends::delete_dividend(&self.pool, input)
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

impl ServerHandler for PortfolioMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mode_note = match (self.access.write_enabled, self.access.destructive_enabled) {
            (false, false) => {
                "Running in read-only mode: write and delete tools are not registered."
            }
            (true, false) => "Write tools are enabled; delete tools are not registered.",
            (false, true) => "Delete tools are enabled; other write tools are not registered.",
            (true, true) => "Write and delete tools are enabled.",
        };
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "portfolio-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(format!(
                "Portfolio Tracker MCP server. Use list_holdings and get_portfolio_snapshot to \
                 read the current portfolio state. Use run_stress_test to simulate market \
                 scenarios. {mode_note}"
            )),
        }
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult {
            next_cursor: None,
            tools: self.visible_tools(),
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        self.ensure_tool_permitted(&request.name)?;
        let call_context = ToolCallContext::new(self, request, context);
        Self::tool_box().call(call_context).await
    }
}

#[cfg(test)]
mod access_tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("open in-memory sqlite")
    }

    const READ_ONLY_TOOL: &str = "list_holdings";
    const WRITE_TOOL: &str = "add_holding";
    const DESTRUCTIVE_TOOL: &str = "delete_holding";

    #[test]
    fn tool_category_classifies_known_tools() {
        assert_eq!(tool_category(READ_ONLY_TOOL), ToolCategory::Read);
        assert_eq!(tool_category("get_portfolio_snapshot"), ToolCategory::Read);
        assert_eq!(tool_category("run_stress_test"), ToolCategory::Read);

        assert_eq!(tool_category(WRITE_TOOL), ToolCategory::Write);
        assert_eq!(tool_category("update_holding"), ToolCategory::Write);
        assert_eq!(tool_category("create_account"), ToolCategory::Write);
        assert_eq!(tool_category("add_dividend"), ToolCategory::Write);
        assert_eq!(tool_category("add_transaction"), ToolCategory::Write);
        assert_eq!(tool_category("add_alert"), ToolCategory::Write);
        assert_eq!(tool_category("reset_alert"), ToolCategory::Write);
        assert_eq!(tool_category("set_config"), ToolCategory::Write);

        assert_eq!(tool_category(DESTRUCTIVE_TOOL), ToolCategory::Destructive);
        assert_eq!(tool_category("delete_dividend"), ToolCategory::Destructive);
        assert_eq!(
            tool_category("delete_transaction"),
            ToolCategory::Destructive
        );
        assert_eq!(tool_category("delete_alert"), ToolCategory::Destructive);
    }

    #[tokio::test]
    async fn read_only_mode_hides_write_and_destructive_tools() {
        let server = PortfolioMcpServer::new(test_pool().await, McpAccess::default());

        let names: Vec<_> = server.visible_tools().into_iter().map(|t| t.name).collect();
        assert!(names.iter().any(|n| n.as_ref() == READ_ONLY_TOOL));
        assert!(!names.iter().any(|n| n.as_ref() == WRITE_TOOL));
        assert!(!names.iter().any(|n| n.as_ref() == DESTRUCTIVE_TOOL));

        assert!(server.ensure_tool_permitted(READ_ONLY_TOOL).is_ok());
        assert!(server.ensure_tool_permitted(WRITE_TOOL).is_err());
        assert!(server.ensure_tool_permitted(DESTRUCTIVE_TOOL).is_err());
    }

    #[tokio::test]
    async fn write_enabled_mode_exposes_write_but_not_destructive_tools() {
        let access = McpAccess {
            write_enabled: true,
            destructive_enabled: false,
        };
        let server = PortfolioMcpServer::new(test_pool().await, access);

        let names: Vec<_> = server.visible_tools().into_iter().map(|t| t.name).collect();
        assert!(names.iter().any(|n| n.as_ref() == READ_ONLY_TOOL));
        assert!(names.iter().any(|n| n.as_ref() == WRITE_TOOL));
        assert!(!names.iter().any(|n| n.as_ref() == DESTRUCTIVE_TOOL));

        assert!(server.ensure_tool_permitted(WRITE_TOOL).is_ok());
        assert!(server.ensure_tool_permitted(DESTRUCTIVE_TOOL).is_err());
    }

    #[tokio::test]
    async fn fully_enabled_mode_exposes_every_tool() {
        let access = McpAccess {
            write_enabled: true,
            destructive_enabled: true,
        };
        let server = PortfolioMcpServer::new(test_pool().await, access);

        let names: Vec<_> = server.visible_tools().into_iter().map(|t| t.name).collect();
        assert!(names.iter().any(|n| n.as_ref() == READ_ONLY_TOOL));
        assert!(names.iter().any(|n| n.as_ref() == WRITE_TOOL));
        assert!(names.iter().any(|n| n.as_ref() == DESTRUCTIVE_TOOL));

        assert!(server.ensure_tool_permitted(WRITE_TOOL).is_ok());
        assert!(server.ensure_tool_permitted(DESTRUCTIVE_TOOL).is_ok());
    }

    #[tokio::test]
    async fn destructive_enabled_without_write_still_gates_write_tools() {
        let access = McpAccess {
            write_enabled: false,
            destructive_enabled: true,
        };
        let server = PortfolioMcpServer::new(test_pool().await, access);

        assert!(server.ensure_tool_permitted(DESTRUCTIVE_TOOL).is_ok());
        assert!(server.ensure_tool_permitted(WRITE_TOOL).is_err());
    }
}
