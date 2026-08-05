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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::error::AppError;
    use crate::stress::{run_stress_test, validate_shocks};
    use crate::types::{
        AccountType, AssetType, Holding, HoldingId, HoldingWithPrice, PortfolioSnapshot,
        StressScenario,
    };

    /// Builds a minimal one-holding snapshot suitable for stress-test unit tests.
    /// The holding is a stock priced at $200 CAD (10 shares × $200 = $2 000 market value).
    fn one_holding_snapshot() -> PortfolioSnapshot {
        let holding = Holding {
            id: HoldingId("AAPL".to_string()),
            symbol: "AAPL".to_string(),
            name: "Apple Inc.".to_string(),
            asset_type: AssetType::Stock,
            account: AccountType::Taxable,
            account_id: None,
            account_name: None,
            quantity: 10.0,
            cost_basis: 150.0,
            currency: "CAD".to_string(), // same as base_currency → no FX shock
            exchange: "NASDAQ".to_string(),
            target_weight: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            indicated_annual_dividend: None,
            indicated_annual_dividend_currency: None,
            dividend_frequency: None,
            maturity_date: None,
        };
        let hwp = HoldingWithPrice {
            holding,
            current_price: 200.0,
            current_price_cad: 200.0,
            market_value_cad: 2_000.0,
            cost_value_cad: 1_500.0,
            gain_loss: 500.0,
            gain_loss_percent: 33.33,
            weight: 100.0,
            target_value: 0.0,
            target_delta_value: 0.0,
            target_delta_percent: 0.0,
            daily_change_percent: 0.0,
            fx_stale: false,
            price_is_stale: false,
        };
        PortfolioSnapshot {
            holdings: vec![hwp],
            total_value: 2_000.0,
            total_cost: 1_500.0,
            total_gain_loss: 500.0,
            total_gain_loss_percent: 33.33,
            daily_pnl: 0.0,
            last_updated: "2024-01-01T00:00:00Z".to_string(),
            base_currency: "CAD".to_string(),
            total_target_weight: 0.0,
            target_cash_delta: 0.0,
            realized_gains: 0.0,
            annual_dividend_income: 0.0,
            requires_cost_basis_selection: false,
        }
    }

    // ── validate_shocks → AppError::Validation mapping ─────────────────────
    //
    // These tests guard the `.map_err(AppError::Validation)` wiring in
    // `run_stress_test_cmd`: an invalid shock must become an
    // `AppError::Validation` *before* `get_portfolio` is ever called.

    #[test]
    fn nan_shock_maps_to_validation_error() {
        let shocks = HashMap::from([("equity".to_string(), f64::NAN)]);
        let err = validate_shocks(&shocks)
            .map_err(AppError::Validation)
            .unwrap_err();
        assert!(
            matches!(err, AppError::Validation(_)),
            "expected AppError::Validation, got {err:?}"
        );
    }

    #[test]
    fn shock_above_max_maps_to_validation_error() {
        let shocks = HashMap::from([("stock".to_string(), 5.01)]); // > +500 %
        let err = validate_shocks(&shocks)
            .map_err(AppError::Validation)
            .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn shock_below_minus_100_maps_to_validation_error() {
        let shocks = HashMap::from([("stock".to_string(), -1.01)]); // < -100 %
        let err = validate_shocks(&shocks)
            .map_err(AppError::Validation)
            .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn valid_empty_shocks_pass_validation() {
        // Boundary: an empty shock map is always valid.
        let result = validate_shocks(&HashMap::new()).map_err(AppError::Validation);
        assert!(result.is_ok());
    }

    // ── happy-path end-to-end through run_stress_test ──────────────────────
    //
    // Uses a synthetic PortfolioSnapshot to exercise the compute path that
    // `run_stress_test_cmd` calls after validation passes. This verifies the
    // full wiring without needing Tauri State or a live database.

    #[test]
    fn stock_shock_applied_correctly_end_to_end() {
        // -30% equity shock on a $2 000 stock holding → stressed value $1 400,
        // total impact -$600.
        let snapshot = one_holding_snapshot();
        let scenario = StressScenario {
            name: "bear market".to_string(),
            shocks: HashMap::from([("stock".to_string(), -0.30)]),
        };

        let result = run_stress_test(&snapshot, &scenario);

        assert_eq!(result.scenario, "bear market");
        assert!(
            (result.current_value - 2_000.0).abs() < 0.01,
            "current_value should be 2000, got {}",
            result.current_value
        );
        assert!(
            (result.stressed_value - 1_400.0).abs() < 0.01,
            "stressed_value should be 1400, got {}",
            result.stressed_value
        );
        assert!(
            (result.total_impact - (-600.0)).abs() < 0.01,
            "total_impact should be -600, got {}",
            result.total_impact
        );
        assert_eq!(result.holding_breakdown.len(), 1);
        assert_eq!(result.holding_breakdown[0].symbol, "AAPL");
    }

    #[test]
    fn zero_shocks_leave_portfolio_value_unchanged() {
        let snapshot = one_holding_snapshot();
        let scenario = StressScenario {
            name: "no shock".to_string(),
            shocks: HashMap::new(),
        };

        let result = run_stress_test(&snapshot, &scenario);

        assert!(
            (result.stressed_value - result.current_value).abs() < 0.01,
            "zero shocks should leave stressed_value == current_value"
        );
        assert!(
            result.total_impact.abs() < 0.01,
            "zero shocks should produce zero total_impact"
        );
    }
}
