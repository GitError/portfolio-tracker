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
        // A holding can lose at most its full value — floor at 0 so shocks
        // beyond -100% (e.g. a -200% scenario) don't drive it negative.
        let stressed_value = (current_value * (1.0 + asset_shock) * (1.0 + fx_shock)).max(0.0);
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

    let total_stressed = total_stressed.max(0.0);
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
    use portfolio_core::types::{AccountType, AssetType, Holding, HoldingId, HoldingWithPrice};
    use std::collections::HashMap;

    fn make_holding(
        symbol: &str,
        asset_type: AssetType,
        currency: &str,
        value: f64,
    ) -> HoldingWithPrice {
        HoldingWithPrice {
            holding: Holding {
                id: HoldingId(symbol.to_string()),
                symbol: symbol.to_string(),
                name: symbol.to_string(),
                asset_type: asset_type.clone(),
                account: if matches!(asset_type, AssetType::Cash) {
                    AccountType::Cash
                } else {
                    AccountType::Taxable
                },
                account_id: None,
                account_name: None,
                quantity: 1.0,
                cost_basis: value,
                currency: currency.to_string(),
                exchange: String::new(),
                target_weight: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
                indicated_annual_dividend: None,
                indicated_annual_dividend_currency: None,
                dividend_frequency: None,
                maturity_date: None,
            },
            current_price: value,
            current_price_cad: value,
            market_value_cad: value,
            cost_value_cad: value,
            gain_loss: 0.0,
            gain_loss_percent: 0.0,
            weight: 1.0,
            target_value: 0.0,
            target_delta_value: 0.0,
            target_delta_percent: 0.0,
            daily_change_percent: 0.0,
            fx_stale: false,
            price_is_stale: false,
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
            total_target_weight: 0.0,
            target_cash_delta: 0.0,
            realized_gains: 0.0,
            annual_dividend_income: 0.0,
            requires_cost_basis_selection: false,
        }
    }

    #[test]
    fn extreme_negative_shock_floors_holding_and_portfolio_at_zero() {
        // Regression guard for #635: a shock beyond -100% (e.g. -200%) must not
        // drive the stressed value negative. A holding can go to $0 but not below.
        let value = 10_000.0;
        let snapshot = make_snapshot(vec![make_holding("BTC", AssetType::Crypto, "CAD", value)]);
        let mut shocks = HashMap::new();
        shocks.insert("crypto".to_string(), -2.0); // -200% shock
        let scenario = StressScenario {
            name: "Beyond total wipeout".to_string(),
            shocks,
        };

        let result = run_stress_test(&snapshot, &scenario);

        assert!(
            result.stressed_value >= 0.0,
            "portfolio stressed_value should not be negative, got {}",
            result.stressed_value
        );
        assert!(
            (result.stressed_value - 0.0).abs() < 0.001,
            "expected stressed_value floored at 0, got {}",
            result.stressed_value
        );
        assert!(
            result.holding_breakdown[0].stressed_value >= 0.0,
            "holding stressed_value should not be negative, got {}",
            result.holding_breakdown[0].stressed_value
        );
        assert!((result.total_impact - (-value)).abs() < 0.001);
    }
}
