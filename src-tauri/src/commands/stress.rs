use tauri::State;

use crate::error::AppError;
use crate::stress::{run_stress_test, validate_shocks};
use crate::types::{StressResult, StressScenario};

use super::{get_portfolio, DbState, HttpClient, RealizedGainsCacheState};

#[tauri::command]
pub async fn run_stress_test_cmd(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
    gains_cache: State<'_, RealizedGainsCacheState>,
    scenario: StressScenario,
) -> Result<StressResult, AppError> {
    validate_shocks(&scenario.shocks).map_err(AppError::Validation)?;
    let snapshot = get_portfolio(db, client, gains_cache).await?;
    Ok(run_stress_test(&snapshot, &scenario))
}
