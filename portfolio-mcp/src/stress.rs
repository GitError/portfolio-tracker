use crate::types::{PortfolioSnapshot, StressHoldingResult, StressResult, StressScenario};

fn fx_shock_key(currency: &str, base_currency: &str) -> String {
    format!(
        "fx_{}_{}",
        currency.to_lowercase(),
        base_currency.to_lowercase()
    )
}

/// Apply a stress scenario to an existing portfolio snapshot and return the
/// impact breakdown.  This is a pure function — it does not modify the snapshot.
pub fn run_stress_test(snapshot: &PortfolioSnapshot, scenario: &StressScenario) -> StressResult {
    let mut holding_results: Vec<StressHoldingResult> = Vec::new();
    let mut total_stressed = 0.0;

    for holding in &snapshot.holdings {
        let asset_type_key = holding.asset_type.as_str();

        // Cash positions are immune to asset-class shocks but still subject to FX shocks.
        let asset_shock = if asset_type_key == "cash" {
            0.0
        } else {
            scenario.shocks.get(asset_type_key).copied().unwrap_or(0.0)
        };

        // FX shock applies only when the holding is not denominated in the portfolio base currency.
        let fx_shock = if !holding
            .currency
            .eq_ignore_ascii_case(&snapshot.base_currency)
        {
            let fx_key = fx_shock_key(&holding.currency, &snapshot.base_currency);
            scenario.shocks.get(&fx_key).copied().unwrap_or(0.0)
        } else {
            0.0
        };

        let current_value = holding.market_value_cad;
        let stressed_value = current_value * (1.0 + asset_shock) * (1.0 + fx_shock);
        let impact = stressed_value - current_value;
        let shock_applied = (1.0 + asset_shock) * (1.0 + fx_shock) - 1.0;

        holding_results.push(StressHoldingResult {
            holding_id: holding.id.clone(),
            symbol: holding.symbol.clone(),
            name: holding.name.clone(),
            current_value,
            stressed_value,
            impact,
            shock_applied,
        });

        total_stressed += stressed_value;
    }

    let current_value = snapshot.total_value;
    let total_impact = total_stressed - current_value;
    let total_impact_percent = if current_value != 0.0 {
        (total_impact / current_value) * 100.0
    } else {
        0.0
    };

    StressResult {
        scenario: scenario.name.clone(),
        current_value,
        stressed_value: total_stressed,
        total_impact,
        total_impact_percent,
        holding_breakdown: holding_results,
    }
}
