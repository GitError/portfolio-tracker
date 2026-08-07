mod db;
mod snapshot;
mod stress;
mod tools;
mod types;
mod validation;

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

/// Read a boolean opt-in flag from the environment. Only the literal value
/// `"true"` enables it (mirrors the `true`/`false` config-value convention
/// used elsewhere in this crate, see `validation.rs`); unset or any other
/// value defaults to disabled.
fn env_flag(name: &str) -> bool {
    std::env::var(name).as_deref() == Ok("true")
}

/// Resolve which mutating tool categories this server instance registers.
///
/// Read-only by default. Write tools (add/update/create/set) require
/// `PORTFOLIO_MCP_WRITE_ENABLED=true`; destructive tools (delete_*) require
/// `PORTFOLIO_MCP_DESTRUCTIVE_WRITE_ENABLED=true`. The two flags are
/// independent so an operator can, for example, allow data entry without
/// allowing deletion.
fn resolve_access() -> tools::McpAccess {
    tools::McpAccess {
        write_enabled: env_flag("PORTFOLIO_MCP_WRITE_ENABLED"),
        destructive_enabled: env_flag("PORTFOLIO_MCP_DESTRUCTIVE_WRITE_ENABLED"),
    }
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

    let access = resolve_access();
    tracing::info!(
        mode = access.mode_label(),
        write_enabled = access.write_enabled,
        destructive_enabled = access.destructive_enabled,
        "portfolio-mcp access mode (opt in via PORTFOLIO_MCP_WRITE_ENABLED=true / \
         PORTFOLIO_MCP_DESTRUCTIVE_WRITE_ENABLED=true)"
    );

    tracing::info!("portfolio-mcp server starting (stdio transport)");

    let server = tools::PortfolioMcpServer::new(pool, access);
    let transport = stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;

    Ok(())
}
