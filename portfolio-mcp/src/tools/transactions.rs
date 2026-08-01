use rmcp::Error as McpError;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    db,
    types::{HoldingId, Transaction, TransactionId, TransactionInput, TransactionType},
    validation,
};

use super::PortfolioMcpServer;

// ── Params ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddTransactionParams {
    /// UUID of the holding this transaction belongs to.
    pub holding_id: String,
    /// Transaction direction: "buy" or "sell".
    pub transaction_type: String,
    /// Number of units bought or sold.
    pub quantity: f64,
    /// Price per unit in the holding's native currency.
    pub price: f64,
    /// ISO 8601 timestamp of when the transaction occurred.
    pub transacted_at: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteTransactionParams {
    /// UUID of the transaction to delete.
    pub id: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list_transactions(pool: &SqlitePool) -> Result<Vec<Transaction>, McpError> {
    db::get_all_transactions(pool)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn add_transaction(
    pool: &SqlitePool,
    params: AddTransactionParams,
) -> Result<Transaction, McpError> {
    let transaction_type = params
        .transaction_type
        .parse::<TransactionType>()
        .map_err(|e| McpError::invalid_params(e, None))?;

    validation::validate_non_empty("holdingId", &params.holding_id)?;
    validation::validate_transaction_fields(params.quantity, params.price)?;

    let input = TransactionInput {
        holding_id: HoldingId(params.holding_id),
        transaction_type,
        quantity: params.quantity,
        price: params.price,
        transacted_at: params.transacted_at,
    };

    db::insert_transaction(pool, input)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn delete_transaction(
    pool: &SqlitePool,
    params: DeleteTransactionParams,
) -> Result<bool, McpError> {
    validation::validate_id("id", &params.id)?;
    let id = TransactionId(params.id);
    db::delete_transaction(pool, &id)
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
        // Table exists but is empty: an unvalidated malformed/empty ID would
        // still run a syntactically valid UPDATE affecting 0 rows (Ok(false)),
        // NOT an error — so this table is required to prove validation, not
        // just a missing-table side effect, is what rejects the bad ID.
        sqlx::query("CREATE TABLE transactions (id TEXT PRIMARY KEY, deleted_at TEXT)")
            .execute(&pool)
            .await
            .expect("create transactions table");
        pool
    }

    #[tokio::test]
    async fn delete_transaction_rejects_empty_id() {
        // Regression guard for #685.
        let pool = test_pool().await;
        let result = delete_transaction(
            &pool,
            DeleteTransactionParams {
                id: "".to_string(),
            },
        )
        .await;
        assert!(result.is_err(), "empty ID must be rejected");
    }

    #[tokio::test]
    async fn delete_transaction_rejects_malformed_id() {
        let pool = test_pool().await;
        let result = delete_transaction(
            &pool,
            DeleteTransactionParams {
                id: "not-a-uuid".to_string(),
            },
        )
        .await;
        assert!(result.is_err(), "malformed ID must be rejected");
    }
}
