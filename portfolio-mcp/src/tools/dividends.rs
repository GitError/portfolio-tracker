use rmcp::Error as McpError;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    db,
    types::{Dividend, DividendId, DividendInput, HoldingId},
    validation,
};

use super::PortfolioMcpServer;

// ── Params ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListDividendsParams {
    /// Optional UUID to filter dividends for a single holding.
    pub holding_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddDividendParams {
    /// UUID of the holding this dividend belongs to.
    pub holding_id: String,
    /// Dividend amount per unit, in the holding's currency.
    pub amount_per_unit: f64,
    /// ISO 4217 currency code; must match the holding's currency.
    pub currency: String,
    /// Ex-dividend date (YYYY-MM-DD).
    pub ex_date: String,
    /// Payment date (YYYY-MM-DD); must not be before ex_date.
    pub pay_date: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteDividendParams {
    /// UUID of the dividend to delete.
    pub id: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list_dividends(
    pool: &SqlitePool,
    params: ListDividendsParams,
) -> Result<Vec<Dividend>, McpError> {
    if let Some(id) = &params.holding_id {
        validation::validate_id("holdingId", id)?;
    }
    db::get_dividends(pool, params.holding_id.as_deref())
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn add_dividend(
    pool: &SqlitePool,
    params: AddDividendParams,
) -> Result<Dividend, McpError> {
    validation::validate_id("holdingId", &params.holding_id)?;
    validation::validate_dividend_fields(
        params.amount_per_unit,
        &params.ex_date,
        &params.pay_date,
    )?;

    let (symbol, holding_currency) = db::get_holding_symbol_and_currency(pool, &params.holding_id)
        .await
        .map_err(PortfolioMcpServer::tool_error)?
        .ok_or_else(|| {
            McpError::invalid_params(format!("Holding {} not found", params.holding_id), None)
        })?;

    if holding_currency.to_uppercase() != params.currency.to_uppercase() {
        return Err(McpError::invalid_params(
            format!(
                "Dividend currency {} does not match holding currency {}",
                params.currency, holding_currency
            ),
            None,
        ));
    }

    let input = DividendInput {
        holding_id: HoldingId(params.holding_id),
        amount_per_unit: params.amount_per_unit,
        currency: params.currency,
        ex_date: params.ex_date,
        pay_date: params.pay_date,
    };

    db::insert_dividend(pool, input, &symbol)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn delete_dividend(
    pool: &SqlitePool,
    params: DeleteDividendParams,
) -> Result<bool, McpError> {
    validation::validate_id("id", &params.id)?;
    let id = DividendId(params.id);
    db::delete_dividend(pool, &id)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    const VALID_HOLDING_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("open in-memory sqlite");
        sqlx::query(
            "CREATE TABLE holdings (id TEXT PRIMARY KEY, symbol TEXT, currency TEXT, \
             deleted_at TEXT)",
        )
        .execute(&pool)
        .await
        .expect("create holdings table");
        sqlx::query(
            "CREATE TABLE dividends (id TEXT PRIMARY KEY, holding_id TEXT, \
             amount_per_unit REAL, currency TEXT, ex_date TEXT, pay_date TEXT, \
             created_at TEXT, deleted_at TEXT)",
        )
        .execute(&pool)
        .await
        .expect("create dividends table");
        pool
    }

    async fn seed_holding(pool: &SqlitePool, currency: &str) {
        sqlx::query("INSERT INTO holdings (id, symbol, currency) VALUES ($1, 'AAPL', $2)")
            .bind(VALID_HOLDING_ID)
            .bind(currency)
            .execute(pool)
            .await
            .expect("seed holding");
    }

    fn make_add_params() -> AddDividendParams {
        AddDividendParams {
            holding_id: VALID_HOLDING_ID.to_string(),
            amount_per_unit: 0.5,
            currency: "USD".to_string(),
            ex_date: "2024-01-01".to_string(),
            pay_date: "2024-01-15".to_string(),
        }
    }

    #[tokio::test]
    async fn delete_dividend_rejects_empty_id() {
        let pool = test_pool().await;
        let result = delete_dividend(&pool, DeleteDividendParams { id: "".to_string() }).await;
        assert!(result.is_err(), "empty ID must be rejected");
    }

    #[tokio::test]
    async fn delete_dividend_rejects_malformed_id() {
        let pool = test_pool().await;
        let result = delete_dividend(
            &pool,
            DeleteDividendParams {
                id: "not-a-uuid".to_string(),
            },
        )
        .await;
        assert!(result.is_err(), "malformed ID must be rejected");
    }

    #[tokio::test]
    async fn add_dividend_rejects_non_positive_amount() {
        let pool = test_pool().await;
        let mut params = make_add_params();
        params.amount_per_unit = 0.0;
        let result = add_dividend(&pool, params).await;
        assert!(result.is_err(), "non-positive amount must be rejected");
    }

    #[tokio::test]
    async fn add_dividend_rejects_non_iso_dates() {
        let pool = test_pool().await;
        let mut params = make_add_params();
        params.ex_date = "01/01/2024".to_string();
        let result = add_dividend(&pool, params).await;
        assert!(result.is_err(), "non-ISO ex_date must be rejected");
    }

    #[tokio::test]
    async fn add_dividend_rejects_pay_date_before_ex_date() {
        let pool = test_pool().await;
        let mut params = make_add_params();
        params.ex_date = "2024-01-15".to_string();
        params.pay_date = "2024-01-01".to_string();
        let result = add_dividend(&pool, params).await;
        assert!(result.is_err(), "pay_date before ex_date must be rejected");
    }

    #[tokio::test]
    async fn add_dividend_rejects_unknown_holding() {
        let pool = test_pool().await;
        let result = add_dividend(&pool, make_add_params()).await;
        assert!(result.is_err(), "unknown holding must be rejected");
    }

    #[tokio::test]
    async fn add_dividend_rejects_currency_mismatch() {
        let pool = test_pool().await;
        seed_holding(&pool, "USD").await;
        let mut params = make_add_params();
        params.currency = "CAD".to_string();
        let result = add_dividend(&pool, params).await;
        assert!(result.is_err(), "mismatched currency must be rejected");
    }

    #[tokio::test]
    async fn add_dividend_persists_and_list_filters_by_holding() {
        let pool = test_pool().await;
        seed_holding(&pool, "USD").await;
        let mut params = make_add_params();
        params.currency = "usd".to_string();
        let created = add_dividend(&pool, params)
            .await
            .expect("add_dividend should succeed");
        assert_eq!(created.symbol, "AAPL");

        let all = list_dividends(&pool, ListDividendsParams { holding_id: None })
            .await
            .expect("list_dividends should succeed");
        assert_eq!(all.len(), 1);

        let filtered = list_dividends(
            &pool,
            ListDividendsParams {
                holding_id: Some(VALID_HOLDING_ID.to_string()),
            },
        )
        .await
        .expect("list_dividends with filter should succeed");
        assert_eq!(filtered.len(), 1);

        let empty = list_dividends(
            &pool,
            ListDividendsParams {
                holding_id: Some("00000000-0000-0000-0000-000000000000".to_string()),
            },
        )
        .await
        .expect("list_dividends with non-matching filter should succeed");
        assert_eq!(empty.len(), 0);
    }
}
