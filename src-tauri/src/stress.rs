use crate::types::{PortfolioSnapshot, StressHoldingResult, StressResult, StressScenario};

pub fn run_stress_test(snapshot: &PortfolioSnapshot, scenario: &StressScenario) -> StressResult {
    let mut holding_results: Vec<StressHoldingResult> = Vec::new();
    let mut total_stressed = 0.0;

    for holding in &snapshot.holdings {
        let asset_type_key = holding.asset_type.as_str().to_string();

        // Asset-level shock (e.g., "stock", "etf", "crypto", "cash")
        let asset_shock = scenario.shocks.get(&asset_type_key).copied().unwrap_or(0.0);

        // FX shock: apply if holding is not in CAD
        let fx_shock = if holding.currency.to_uppercase() != "CAD" {
            let fx_key = format!("fx_{}_cad", holding.currency.to_lowercase());
            scenario.shocks.get(&fx_key).copied().unwrap_or(0.0)
        } else {
            0.0
        };

        let current_value = holding.market_value_cad;
        let stressed_value = current_value * (1.0 + asset_shock) * (1.0 + fx_shock);
        let impact = stressed_value - current_value;

        // Combined shock for display
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
