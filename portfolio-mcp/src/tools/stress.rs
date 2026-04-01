use rmcp::Error as McpError;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::types::{StressResult, StressScenario};

use super::portfolio;

// ── Params ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StressTestParams {
    /// Name for this scenario (e.g. "Bear Market").
    pub name: String,
    /// Map of shock keys to fractional multipliers.
    ///
    /// Supported keys:
    /// - `"stock"`, `"etf"`, `"crypto"` — asset-class shocks
    /// - `"fx_usd_cad"`, `"fx_eur_cad"`, etc. — FX shocks (lower-cased `<from>_<to>`)
    ///
    /// Example: `{"stock": -0.20, "crypto": -0.50, "fx_usd_cad": 0.05}`
    pub shocks: HashMap<String, f64>,
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn run_stress_test(
    pool: &SqlitePool,
    params: StressTestParams,
) -> Result<StressResult, McpError> {
    // Reuse the portfolio snapshot builder so the stress engine always operates
    // on the same view of the data as the main Tauri app.
    let snapshot = portfolio::get_portfolio_snapshot(pool).await?;

    let scenario = StressScenario {
        name: params.name,
        shocks: params.shocks,
    };

    let result = crate::stress::run_stress_test(&snapshot, &scenario);
    Ok(result)
}
