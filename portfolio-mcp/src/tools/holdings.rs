use rmcp::Error as McpError;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    db,
    types::{AccountType, AssetType, Holding, HoldingId, HoldingInput},
    validation,
};

use super::PortfolioMcpServer;

// ── Params ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddHoldingParams {
    /// Ticker symbol (e.g. "AAPL", "BTC-USD", "CASH-CAD").
    pub symbol: String,
    /// Human-readable name.
    pub name: String,
    /// Asset class: "stock", "etf", "crypto", or "cash".
    pub asset_type: String,
    /// Account type: "tfsa", "rrsp", "fhsa", "taxable", "crypto", "cash", or "other".
    pub account: String,
    /// Optional explicit account UUID (overrides account-type lookup).
    pub account_id: Option<String>,
    /// Number of units held.
    pub quantity: f64,
    /// Average cost per unit in the holding's native currency.
    pub cost_basis: f64,
    /// ISO 4217 currency code (e.g. "CAD", "USD").
    pub currency: String,
    /// Exchange identifier (e.g. "TSX", "NASDAQ").
    pub exchange: String,
    /// Target portfolio weight as a percentage (0–100). Omit to leave unset.
    pub target_weight: Option<f64>,
    /// Indicated annual dividend per unit in the dividend currency.
    pub indicated_annual_dividend: Option<f64>,
    pub indicated_annual_dividend_currency: Option<String>,
    /// Dividend frequency: "monthly", "quarterly", "semi-annual", "annual", "irregular".
    pub dividend_frequency: Option<String>,
    /// Maturity date for fixed-income positions (ISO 8601 date string).
    pub maturity_date: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateHoldingParams {
    /// UUID of the holding to update.
    pub id: String,
    /// Ticker symbol (e.g. "AAPL", "BTC-USD", "CASH-CAD").
    pub symbol: String,
    /// Human-readable name.
    pub name: String,
    /// Asset class: "stock", "etf", "crypto", or "cash".
    pub asset_type: String,
    /// Account type: "tfsa", "rrsp", "fhsa", "taxable", "crypto", "cash", or "other".
    pub account: String,
    /// Optional explicit account UUID (overrides account-type lookup).
    pub account_id: Option<String>,
    /// Number of units held.
    pub quantity: f64,
    /// Average cost per unit in the holding's native currency.
    pub cost_basis: f64,
    /// ISO 4217 currency code (e.g. "CAD", "USD").
    pub currency: String,
    /// Exchange identifier (e.g. "TSX", "NASDAQ").
    pub exchange: String,
    /// Target portfolio weight as a percentage (0–100). Omit to leave unset.
    pub target_weight: Option<f64>,
    /// Indicated annual dividend per unit in the dividend currency.
    pub indicated_annual_dividend: Option<f64>,
    pub indicated_annual_dividend_currency: Option<String>,
    /// Dividend frequency: "monthly", "quarterly", "semi-annual", "annual", "irregular".
    pub dividend_frequency: Option<String>,
    /// Maturity date for fixed-income positions (ISO 8601 date string).
    pub maturity_date: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteHoldingParams {
    /// UUID of the holding to delete.
    pub id: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list_holdings(pool: &SqlitePool) -> Result<Vec<Holding>, McpError> {
    db::get_all_holdings(pool)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn add_holding(pool: &SqlitePool, params: AddHoldingParams) -> Result<Holding, McpError> {
    let asset_type = params
        .asset_type
        .parse::<AssetType>()
        .map_err(|e| McpError::invalid_params(e, None))?;

    let account = params
        .account
        .parse::<AccountType>()
        .map_err(|e| McpError::invalid_params(e, None))?;

    validation::validate_non_empty("symbol", &params.symbol)?;
    validation::validate_non_empty("name", &params.name)?;
    let currency =
        validation::validate_holding_fields(params.quantity, params.cost_basis, &params.currency)?;
    validation::validate_target_weight(params.target_weight)?;
    validation::validate_holding_dividend_fields(
        params.indicated_annual_dividend,
        params.dividend_frequency.as_deref(),
        params.maturity_date.as_deref(),
    )?;

    if let Some(target_weight) = params.target_weight {
        let existing_sum = db::sum_target_weights(pool, None)
            .await
            .map_err(PortfolioMcpServer::tool_error)?;
        validation::validate_target_weight_budget(Some(target_weight), existing_sum)?;
    }

    let input = HoldingInput {
        symbol: params.symbol,
        name: params.name,
        asset_type,
        account,
        account_id: params.account_id,
        quantity: params.quantity,
        cost_basis: params.cost_basis,
        currency,
        exchange: params.exchange,
        target_weight: params.target_weight,
        indicated_annual_dividend: params.indicated_annual_dividend,
        indicated_annual_dividend_currency: params.indicated_annual_dividend_currency,
        dividend_frequency: params.dividend_frequency,
        maturity_date: params.maturity_date,
    };

    db::insert_holding(pool, input)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn update_holding(
    pool: &SqlitePool,
    params: UpdateHoldingParams,
) -> Result<Holding, McpError> {
    validation::validate_id("id", &params.id)?;

    let asset_type = params
        .asset_type
        .parse::<AssetType>()
        .map_err(|e| McpError::invalid_params(e, None))?;

    let account = params
        .account
        .parse::<AccountType>()
        .map_err(|e| McpError::invalid_params(e, None))?;

    validation::validate_non_empty("symbol", &params.symbol)?;
    validation::validate_non_empty("name", &params.name)?;
    let currency =
        validation::validate_holding_fields(params.quantity, params.cost_basis, &params.currency)?;
    validation::validate_target_weight(params.target_weight)?;
    validation::validate_holding_dividend_fields(
        params.indicated_annual_dividend,
        params.dividend_frequency.as_deref(),
        params.maturity_date.as_deref(),
    )?;

    if let Some(target_weight) = params.target_weight {
        let existing_sum = db::sum_target_weights(pool, Some(params.id.as_str()))
            .await
            .map_err(PortfolioMcpServer::tool_error)?;
        validation::validate_target_weight_budget(Some(target_weight), existing_sum)?;
    }

    let existing = db::get_holding_by_id(pool, &params.id)
        .await
        .map_err(PortfolioMcpServer::tool_error)?
        .ok_or_else(|| {
            McpError::invalid_params(format!("Holding {} not found", params.id), None)
        })?;

    let holding = Holding {
        id: HoldingId(params.id),
        symbol: params.symbol,
        name: params.name,
        asset_type,
        account,
        account_id: params.account_id,
        account_name: existing.account_name,
        quantity: params.quantity,
        cost_basis: params.cost_basis,
        currency,
        exchange: params.exchange,
        target_weight: params.target_weight,
        created_at: existing.created_at,
        updated_at: existing.updated_at,
        indicated_annual_dividend: params.indicated_annual_dividend,
        indicated_annual_dividend_currency: params.indicated_annual_dividend_currency,
        dividend_frequency: params.dividend_frequency,
        maturity_date: params.maturity_date,
    };

    db::update_holding(pool, holding)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn delete_holding(
    pool: &SqlitePool,
    params: DeleteHoldingParams,
) -> Result<bool, McpError> {
    validation::validate_id("id", &params.id)?;
    let id = HoldingId(params.id);
    db::delete_holding(pool, &id)
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
        sqlx::query("CREATE TABLE holdings (id TEXT PRIMARY KEY, deleted_at TEXT)")
            .execute(&pool)
            .await
            .expect("create holdings table");
        pool
    }

    #[tokio::test]
    async fn delete_holding_rejects_empty_id() {
        // Regression guard for #685: an empty/malformed ID must be rejected
        // before it ever reaches the database, mirroring the Tauri command.
        let pool = test_pool().await;
        let result = delete_holding(
            &pool,
            DeleteHoldingParams {
                id: "".to_string(),
            },
        )
        .await;
        assert!(result.is_err(), "empty ID must be rejected");
    }

    #[tokio::test]
    async fn delete_holding_rejects_malformed_id() {
        let pool = test_pool().await;
        let result = delete_holding(
            &pool,
            DeleteHoldingParams {
                id: "not-a-uuid".to_string(),
            },
        )
        .await;
        assert!(result.is_err(), "malformed ID must be rejected");
    }

    const VALID_HOLDING_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

    fn make_update_params(id: &str) -> UpdateHoldingParams {
        UpdateHoldingParams {
            id: id.to_string(),
            symbol: "AAPL".to_string(),
            name: "Apple Inc.".to_string(),
            asset_type: "stock".to_string(),
            account: "taxable".to_string(),
            account_id: None,
            quantity: 5.0,
            cost_basis: 120.0,
            currency: "USD".to_string(),
            exchange: "NASDAQ".to_string(),
            target_weight: None,
            indicated_annual_dividend: None,
            indicated_annual_dividend_currency: None,
            dividend_frequency: None,
            maturity_date: None,
        }
    }

    #[tokio::test]
    async fn update_holding_rejects_empty_id() {
        let pool = test_pool().await;
        let result = update_holding(&pool, make_update_params("")).await;
        assert!(result.is_err(), "empty ID must be rejected");
    }

    #[tokio::test]
    async fn update_holding_rejects_malformed_id() {
        let pool = test_pool().await;
        let result = update_holding(&pool, make_update_params("not-a-uuid")).await;
        assert!(result.is_err(), "malformed ID must be rejected");
    }

    #[tokio::test]
    async fn update_holding_rejects_non_positive_quantity() {
        let pool = test_pool().await;
        let mut params = make_update_params(VALID_HOLDING_ID);
        params.quantity = 0.0;
        let result = update_holding(&pool, params).await;
        assert!(result.is_err(), "non-positive quantity must be rejected");
    }

    async fn test_pool_full() -> SqlitePool {
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
        pool
    }

    #[tokio::test]
    async fn update_holding_rejects_when_not_found() {
        let pool = test_pool_full().await;
        let result = update_holding(&pool, make_update_params(VALID_HOLDING_ID)).await;
        assert!(
            result.is_err(),
            "updating a nonexistent holding must be rejected"
        );
    }

    #[tokio::test]
    async fn update_holding_updates_existing_holding() {
        let pool = test_pool_full().await;
        let created_at = "2024-01-01T00:00:00Z";
        sqlx::query(
            "INSERT INTO holdings (id, symbol, name, asset_type, account, account_id,
                quantity, cost_basis, currency, exchange, target_weight, created_at, updated_at)
             VALUES ($1, 'AAPL', 'Apple', 'stock', 'taxable', NULL, 1.0, 100.0, 'CAD',
                     'NASDAQ', NULL, $2, $2)",
        )
        .bind(VALID_HOLDING_ID)
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("seed holding");

        let mut params = make_update_params(VALID_HOLDING_ID);
        params.currency = "  usd ".to_string();
        let updated = update_holding(&pool, params)
            .await
            .expect("update_holding should succeed");

        assert_eq!(updated.quantity, 5.0);
        assert_eq!(updated.cost_basis, 120.0);
        assert_eq!(updated.name, "Apple Inc.");
        assert_eq!(updated.currency, "USD", "currency must be normalized");
        assert_eq!(
            updated.created_at, created_at,
            "created_at must be preserved from the existing row"
        );
    }
}
