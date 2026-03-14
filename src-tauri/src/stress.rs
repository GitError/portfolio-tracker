use crate::types::{PortfolioSnapshot, StressHoldingResult, StressResult, StressScenario};

#[cfg(test)]
use crate::types::{AccountType, AssetType, HoldingWithPrice};

fn fx_shock_key(currency: &str, base_currency: &str) -> String {
    format!(
        "fx_{}_{}",
        currency.to_lowercase(),
        base_currency.to_lowercase()
    )
}

pub fn run_stress_test(snapshot: &PortfolioSnapshot, scenario: &StressScenario) -> StressResult {
    let mut holding_results: Vec<StressHoldingResult> = Vec::new();
    let mut total_stressed = 0.0;

    for holding in &snapshot.holdings {
        let asset_type_key = holding.asset_type.as_str().to_string();

        // Asset-level shock (e.g., "stock", "etf", "crypto", "cash")
        let asset_shock = scenario.shocks.get(&asset_type_key).copied().unwrap_or(0.0);

        // FX shock: apply if holding is not in the portfolio base currency.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_holding(
        symbol: &str,
        asset_type: AssetType,
        currency: &str,
        value: f64,
    ) -> HoldingWithPrice {
        HoldingWithPrice {
            id: symbol.to_string(),
            symbol: symbol.to_string(),
            name: symbol.to_string(),
            asset_type: asset_type.clone(),
            account: if matches!(asset_type, AssetType::Cash) {
                AccountType::Cash
            } else {
                AccountType::Taxable
            },
            quantity: 1.0,
            cost_basis: value,
            currency: currency.to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            current_price: value,
            current_price_cad: value,
            market_value_cad: value,
            cost_value_cad: value,
            gain_loss: 0.0,
            gain_loss_percent: 0.0,
            weight: 1.0,
            daily_change_percent: 0.0,
        }
    }

    fn make_snapshot(holdings: Vec<HoldingWithPrice>) -> PortfolioSnapshot {
        let total = holdings.iter().map(|h| h.market_value_cad).sum();
        PortfolioSnapshot {
            holdings,
            total_value: total,
            total_cost: total,
            total_gain_loss: 0.0,
            total_gain_loss_percent: 0.0,
            daily_pnl: 0.0,
            last_updated: "2024-01-01T00:00:00Z".to_string(),
            base_currency: "CAD".to_string(),
        }
    }

    #[test]
    fn zero_shocks_return_unchanged_values() {
        let snapshot = make_snapshot(vec![
            make_holding("AAPL", AssetType::Stock, "USD", 10_000.0),
            make_holding("BTC", AssetType::Crypto, "CAD", 5_000.0),
        ]);
        let scenario = StressScenario {
            name: "Zero".to_string(),
            shocks: HashMap::new(),
        };

        let result = run_stress_test(&snapshot, &scenario);

        assert!(
            (result.total_impact).abs() < 0.001,
            "Zero shocks should produce zero impact"
        );
        assert_eq!(result.holding_breakdown.len(), 2);
        for h in &result.holding_breakdown {
            assert!((h.impact).abs() < 0.001);
        }
    }

    #[test]
    fn stock_shock_applies_correctly() {
        let value = 10_000.0;
        let snapshot = make_snapshot(vec![make_holding("AAPL", AssetType::Stock, "CAD", value)]);
        let mut shocks = HashMap::new();
        shocks.insert("stock".to_string(), -0.20);
        let scenario = StressScenario {
            name: "Bear".to_string(),
            shocks,
        };

        let result = run_stress_test(&snapshot, &scenario);

        let expected_stressed = value * 0.80;
        assert!((result.stressed_value - expected_stressed).abs() < 0.001);
        assert!((result.total_impact - (-2_000.0)).abs() < 0.001);
    }

    #[test]
    fn fx_shock_applies_to_non_base_holdings() {
        let value = 10_000.0;
        let snapshot = make_snapshot(vec![make_holding("AAPL", AssetType::Stock, "USD", value)]);
        let mut shocks = HashMap::new();
        shocks.insert("stock".to_string(), -0.10);
        shocks.insert("fx_usd_cad".to_string(), 0.05);
        let scenario = StressScenario {
            name: "Mixed".to_string(),
            shocks,
        };

        let result = run_stress_test(&snapshot, &scenario);

        // stressed = 10000 * 0.90 * 1.05 = 9450
        let expected = value * 0.90 * 1.05;
        assert!((result.stressed_value - expected).abs() < 0.001);
    }

    #[test]
    fn base_currency_holdings_ignore_fx_shock() {
        let value = 5_000.0;
        let snapshot = make_snapshot(vec![make_holding("RY.TO", AssetType::Stock, "CAD", value)]);
        let mut shocks = HashMap::new();
        shocks.insert("fx_usd_cad".to_string(), 0.15); // should not affect CAD holding
        let scenario = StressScenario {
            name: "FX only".to_string(),
            shocks,
        };

        let result = run_stress_test(&snapshot, &scenario);

        assert!((result.total_impact).abs() < 0.001);
    }

    #[test]
    fn fx_shock_uses_snapshot_base_currency() {
        let value = 8_000.0;
        let mut snapshot = make_snapshot(vec![
            make_holding("RY.TO", AssetType::Stock, "CAD", value),
            make_holding("MSFT", AssetType::Stock, "USD", value),
        ]);
        snapshot.base_currency = "USD".to_string();

        let mut shocks = HashMap::new();
        shocks.insert("fx_cad_usd".to_string(), -0.10);
        let scenario = StressScenario {
            name: "USD Base".to_string(),
            shocks,
        };

        let result = run_stress_test(&snapshot, &scenario);

        assert!((result.holding_breakdown[0].stressed_value - 7_200.0).abs() < 0.001);
        assert!((result.holding_breakdown[1].stressed_value - 8_000.0).abs() < 0.001);
    }

    #[test]
    fn empty_snapshot_returns_zero() {
        let snapshot = make_snapshot(vec![]);
        let scenario = StressScenario {
            name: "Empty".to_string(),
            shocks: HashMap::new(),
        };
        let result = run_stress_test(&snapshot, &scenario);
        assert_eq!(result.total_impact, 0.0);
        assert_eq!(result.total_impact_percent, 0.0);
    }
}
