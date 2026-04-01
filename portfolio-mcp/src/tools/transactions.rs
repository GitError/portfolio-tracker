use rmcp::Error as McpError;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    db,
    types::{HoldingId, Transaction, TransactionId, TransactionInput, TransactionType},
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
    let id = TransactionId(params.id);
    db::delete_transaction(pool, &id)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}
