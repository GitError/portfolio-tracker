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

    // If the user has never explicitly chosen a cost-basis method, flag the
    // snapshot so the caller can prompt for an explicit selection before
    // displaying realized gains — mirrors `get_portfolio_impl` in
    // `src-tauri/src/commands/portfolio.rs`.
    let requires_cost_basis_selection = db::get_config(pool, "cost_basis_method")
        .await
        .map_err(PortfolioMcpServer::tool_error)?
        .is_none();

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
    let mut snapshot = snapshot::build_portfolio_snapshot(
        &holdings,
        &cached_prices,
        &cached_fx,
        &base_currency,
        last_updated,
        0.0,
        0.0,
    );
    snapshot.requires_cost_basis_selection = requires_cost_basis_selection;

    Ok(snapshot)
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
        sqlx::query(
            "CREATE TABLE accounts (id TEXT PRIMARY KEY, name TEXT, type TEXT, \
             institution TEXT, created_at TEXT)",
        )
        .execute(&pool)
        .await
        .expect("create accounts table");
        sqlx::query(
            "CREATE TABLE holdings (
                id TEXT PRIMARY KEY, symbol TEXT, name TEXT, asset_type TEXT, account TEXT,
                account_id TEXT, quantity REAL, cost_basis REAL, currency TEXT, exchange TEXT,
                target_weight REAL, created_at TEXT, updated_at TEXT,
                indicated_annual_dividend REAL, indicated_annual_dividend_currency TEXT,
                dividend_frequency TEXT, maturity_date TEXT, deleted_at TEXT
            )",
        )
        .execute(&pool)
        .await
        .expect("create holdings table");
        sqlx::query(
            "CREATE TABLE price_cache (symbol TEXT PRIMARY KEY, price REAL, currency TEXT, \
             change REAL, change_percent REAL, updated_at TEXT, open REAL, \
             previous_close REAL, volume INTEGER)",
        )
        .execute(&pool)
        .await
        .expect("create price_cache table");
        sqlx::query("CREATE TABLE fx_rates (pair TEXT PRIMARY KEY, rate REAL, updated_at TEXT)")
            .execute(&pool)
            .await
            .expect("create fx_rates table");
        pool
    }

    #[tokio::test]
    async fn get_portfolio_snapshot_requires_cost_basis_selection_when_unset() {
        // Regression guard for #693: requiresCostBasisSelection was always
        // false in the MCP snapshot, even when the user never chose a method.
        let pool = test_pool().await;
        let snapshot = get_portfolio_snapshot(&pool)
            .await
            .expect("snapshot should succeed");
        assert!(snapshot.requires_cost_basis_selection);
    }

    #[tokio::test]
    async fn get_portfolio_snapshot_does_not_require_cost_basis_selection_when_set() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO app_config (key, value) VALUES ('cost_basis_method', 'avco')")
            .execute(&pool)
            .await
            .expect("seed cost_basis_method");
        let snapshot = get_portfolio_snapshot(&pool)
            .await
            .expect("snapshot should succeed");
        assert!(!snapshot.requires_cost_basis_selection);
    }
}
