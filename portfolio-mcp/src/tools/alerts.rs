use rmcp::Error as McpError;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    db,
    types::{AlertDirection, AlertId, PriceAlert, PriceAlertInput},
    validation,
};

use super::PortfolioMcpServer;

// ── Params ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddAlertParams {
    /// Ticker symbol to watch (e.g. "AAPL").
    pub symbol: String,
    /// Alert direction: "above" fires when price rises above threshold,
    /// "below" fires when price drops below threshold.
    pub direction: String,
    /// Price threshold in the specified currency.
    pub threshold: f64,
    /// ISO 4217 currency code for the threshold (e.g. "USD").
    pub currency: String,
    /// Optional free-text note for this alert.
    pub note: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteAlertParams {
    /// UUID of the alert to delete.
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ResetAlertParams {
    /// UUID of the triggered alert to reset.
    pub id: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list_alerts(pool: &SqlitePool) -> Result<Vec<PriceAlert>, McpError> {
    db::get_alerts(pool)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn add_alert(pool: &SqlitePool, params: AddAlertParams) -> Result<PriceAlert, McpError> {
    let direction = params
        .direction
        .parse::<AlertDirection>()
        .map_err(|e| McpError::invalid_params(e, None))?;

    validation::validate_non_empty("symbol", &params.symbol)?;
    validation::validate_alert_threshold(params.threshold)?;
    validation::validate_alert_currency(&params.currency)?;
    validation::validate_alert_note(&params.note)?;

    let input = PriceAlertInput {
        symbol: params.symbol,
        direction,
        threshold: params.threshold,
        currency: params.currency,
        note: params.note,
    };

    db::insert_alert(pool, input)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn delete_alert(pool: &SqlitePool, params: DeleteAlertParams) -> Result<bool, McpError> {
    validation::validate_id("id", &params.id)?;
    let id = AlertId(params.id);
    db::delete_alert(pool, &id)
        .await
        .map_err(PortfolioMcpServer::tool_error)
}

pub async fn reset_alert(pool: &SqlitePool, params: ResetAlertParams) -> Result<bool, McpError> {
    validation::validate_id("id", &params.id)?;
    let id = AlertId(params.id);
    db::reset_alert(pool, &id)
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
        // still run a syntactically valid query affecting 0 rows (Ok(false)),
        // NOT an error — so this table is required to prove validation, not
        // just a missing-table side effect, is what rejects the bad ID.
        sqlx::query("CREATE TABLE price_alerts (id TEXT PRIMARY KEY, triggered INTEGER)")
            .execute(&pool)
            .await
            .expect("create price_alerts table");
        pool
    }

    #[tokio::test]
    async fn delete_alert_rejects_empty_id() {
        // Regression guard for #685.
        let pool = test_pool().await;
        let result = delete_alert(&pool, DeleteAlertParams { id: "".to_string() }).await;
        assert!(result.is_err(), "empty ID must be rejected");
    }

    #[tokio::test]
    async fn delete_alert_rejects_malformed_id() {
        let pool = test_pool().await;
        let result = delete_alert(
            &pool,
            DeleteAlertParams {
                id: "not-a-uuid".to_string(),
            },
        )
        .await;
        assert!(result.is_err(), "malformed ID must be rejected");
    }

    #[tokio::test]
    async fn reset_alert_rejects_empty_id() {
        let pool = test_pool().await;
        let result = reset_alert(&pool, ResetAlertParams { id: "".to_string() }).await;
        assert!(result.is_err(), "empty ID must be rejected");
    }

    #[tokio::test]
    async fn reset_alert_rejects_malformed_id() {
        let pool = test_pool().await;
        let result = reset_alert(
            &pool,
            ResetAlertParams {
                id: "not-a-uuid".to_string(),
            },
        )
        .await;
        assert!(result.is_err(), "malformed ID must be rejected");
    }
}
