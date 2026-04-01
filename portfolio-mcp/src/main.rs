mod db;
mod snapshot;
mod stress;
mod tools;
mod types;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};

/// Resolve the path to the portfolio SQLite database.
///
/// Priority order:
/// 1. `PORTFOLIO_DB_PATH` environment variable.
/// 2. macOS default: `~/Library/Application Support/com.portfolio-tracker.app/portfolio.db`
fn db_path() -> String {
    if let Ok(p) = std::env::var("PORTFOLIO_DB_PATH") {
        return p;
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    format!(
        "{}/Library/Application Support/com.portfolio-tracker.app/portfolio.db",
        home
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    // MCP uses stdout for the JSON-RPC protocol; log to stderr so we don't
    // corrupt the transport stream.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "portfolio_mcp=info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let path = db_path();
    tracing::info!(%path, "opening portfolio database");

    let pool = db::open_pool(&path).await.map_err(|e| {
        tracing::error!(%e, %path, "failed to open database");
        e
    })?;

    tracing::info!("portfolio-mcp server starting (stdio transport)");

    let server = tools::PortfolioMcpServer::new(pool);
    let transport = stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;

    Ok(())
}
