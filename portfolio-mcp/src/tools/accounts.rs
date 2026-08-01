use rmcp::Error as McpError;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{db, types::Account, validation};

use super::PortfolioMcpServer;

// ── Params ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountParams {
    /// Human-readable account name.
    pub name: String,
    /// Account type: "tfsa", "rrsp", "fhsa", "taxable", "crypto", "cash", or "other".
    pub account_type: String,
    /// Optional institution name (e.g. "Questrade").
    pub institution: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list_accounts(pool: &SqlitePool) -> Result<Vec<Account>, McpError> {
    db::get_accounts(pool)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn create_account(
    pool: &SqlitePool,
    params: CreateAccountParams,
) -> Result<Account, McpError> {
    let name = validation::validate_account_fields(&params.name, &params.account_type)?;
    db::insert_account(
        pool,
        &name,
        &params.account_type,
        params.institution.as_deref(),
    )
    .await
    .map_err(PortfolioMcpServer::tool_error)
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
        sqlx::query(
            "CREATE TABLE accounts (id TEXT PRIMARY KEY, name TEXT, type TEXT, \
             institution TEXT, created_at TEXT)",
        )
        .execute(&pool)
        .await
        .expect("create accounts table");
        pool
    }

    #[tokio::test]
    async fn create_account_rejects_empty_name() {
        let pool = test_pool().await;
        let result = create_account(
            &pool,
            CreateAccountParams {
                name: "   ".to_string(),
                account_type: "tfsa".to_string(),
                institution: None,
            },
        )
        .await;
        assert!(result.is_err(), "empty account name must be rejected");
    }

    #[tokio::test]
    async fn create_account_rejects_unknown_type() {
        let pool = test_pool().await;
        let result = create_account(
            &pool,
            CreateAccountParams {
                name: "My Account".to_string(),
                account_type: "not-a-type".to_string(),
                institution: None,
            },
        )
        .await;
        assert!(result.is_err(), "unknown account type must be rejected");
    }

    #[tokio::test]
    async fn create_account_persists_and_list_returns_it() {
        let pool = test_pool().await;
        let created = create_account(
            &pool,
            CreateAccountParams {
                name: "  My TFSA  ".to_string(),
                account_type: "tfsa".to_string(),
                institution: Some("Questrade".to_string()),
            },
        )
        .await
        .expect("create_account should succeed");
        assert_eq!(created.name, "My TFSA", "name must be trimmed");

        let listed = list_accounts(&pool)
            .await
            .expect("list_accounts should succeed");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);
        assert_eq!(listed[0].institution.as_deref(), Some("Questrade"));
    }
}
