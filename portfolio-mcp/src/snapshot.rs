use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::types::{FxRate, Holding, HoldingWithPrice, PortfolioSnapshot, PriceData};

const PRICE_STALE_SECS: i64 = 24 * 3600;

/// Convert `amount` from `from_currency` into `base` using cached FX rates.
/// Returns `None` when no matching rate is found.
fn convert_to_base(amount: f64, from_currency: &str, base: &str, rates: &[FxRate]) -> Option<f64> {
    let from_upper = from_currency.to_uppercase();
    let base_upper = base.to_uppercase();

    if from_upper == base_upper {
        return Some(amount);
    }

    let direct_pair = format!("{from_upper}{base_upper}");
    if let Some(rate) = rates.iter().find(|r| r.pair == direct_pair) {
        return Some(amount * rate.rate);
    }

    // Try the inverted pair.
    let inverted_pair = format!("{base_upper}{from_upper}");
    if let Some(rate) = rates.iter().find(|r| r.pair == inverted_pair) {
        if rate.rate != 0.0 {
            return Some(amount / rate.rate);
        }
    }

    None
}

/// Build a `PortfolioSnapshot` from raw holdings, cached prices, and FX rates.
/// All monetary values are expressed in `base_currency`.
pub fn build_portfolio_snapshot(
    holdings: &[Holding],
    cached_prices: &[PriceData],
    cached_fx: &[FxRate],
    base_currency: &str,
    last_updated: String,
    realized_gains: f64,
    annual_dividend_income: f64,
) -> PortfolioSnapshot {
    if holdings.is_empty() {
        return PortfolioSnapshot {
            holdings: vec![],
            total_value: 0.0,
            total_cost: 0.0,
            total_gain_loss: 0.0,
            total_gain_loss_percent: 0.0,
            daily_pnl: 0.0,
            last_updated,
            base_currency: base_currency.to_string(),
            total_target_weight: 0.0,
            target_cash_delta: 0.0,
            realized_gains,
            annual_dividend_income,
            requires_cost_basis_selection: false,
        };
    }

    let price_map: HashMap<String, &PriceData> = cached_prices
        .iter()
        .map(|p| (p.symbol.clone(), p))
        .collect();

    let fx_map: HashMap<String, &FxRate> = cached_fx.iter().map(|r| (r.pair.clone(), r)).collect();

    let mut holdings_with_price: Vec<HoldingWithPrice> = Vec::new();
    let mut total_value = 0.0f64;
    let mut total_cost = 0.0f64;
    let mut daily_pnl = 0.0f64;

    for holding in holdings {
        let is_cash = holding.asset_type.as_str() == "cash";

        let (current_price, change_percent, price_is_stale) = if is_cash {
            (1.0f64, 0.0f64, false)
        } else {
            match price_map.get(&holding.symbol) {
                Some(p) => {
                    let stale = DateTime::parse_from_rfc3339(&p.updated_at)
                        .ok()
                        .map(|t| {
                            Utc::now()
                                .signed_duration_since(t.with_timezone(&Utc))
                                .num_seconds()
                                > PRICE_STALE_SECS
                        })
                        .unwrap_or(true);
                    (p.price, p.change_percent, stale)
                }
                None => (holding.cost_basis, 0.0, true),
            }
        };

        let fx_pair = format!(
            "{}{}",
            holding.currency.to_uppercase(),
            base_currency.to_uppercase()
        );

        let (fx_rate, fx_stale) = if holding.currency.eq_ignore_ascii_case(base_currency) {
            (1.0, false)
        } else {
            match fx_map
                .get(&fx_pair)
                .map(|r| r.rate)
                .or_else(|| convert_to_base(1.0, &holding.currency, base_currency, cached_fx))
            {
                Some(rate) => (rate, false),
                None => {
                    tracing::warn!(
                        symbol = %holding.symbol,
                        currency = %holding.currency,
                        base = %base_currency,
                        "FX rate unavailable — marking holding as fx_stale"
                    );
                    (1.0, true)
                }
            }
        };

        let current_price_cad = current_price * fx_rate;
        let market_value_cad = holding.quantity * current_price_cad;
        let cost_value_cad = holding.quantity * holding.cost_basis * fx_rate;
        let gain_loss = market_value_cad - cost_value_cad;
        let gain_loss_percent = if cost_value_cad != 0.0 {
            (gain_loss / cost_value_cad) * 100.0
        } else {
            0.0
        };

        total_value += market_value_cad;
        total_cost += cost_value_cad;

        // Exclude intraday purchases from daily PnL.
        let today_utc = Utc::now().date_naive().to_string();
        let created_date_utc = holding.created_at.get(..10).unwrap_or("");
        if !created_date_utc.is_empty() && created_date_utc < today_utc.as_str() {
            daily_pnl += market_value_cad * (change_percent / 100.0);
        }

        holdings_with_price.push(HoldingWithPrice {
            id: holding.id.clone(),
            symbol: holding.symbol.clone(),
            name: holding.name.clone(),
            asset_type: holding.asset_type.clone(),
            account: holding.account.clone(),
            account_id: holding.account_id.clone(),
            account_name: holding.account_name.clone(),
            quantity: holding.quantity,
            cost_basis: holding.cost_basis,
            currency: holding.currency.clone(),
            exchange: holding.exchange.clone(),
            target_weight: holding.target_weight,
            created_at: holding.created_at.clone(),
            updated_at: holding.updated_at.clone(),
            indicated_annual_dividend: holding.indicated_annual_dividend,
            indicated_annual_dividend_currency: holding.indicated_annual_dividend_currency.clone(),
            dividend_frequency: holding.dividend_frequency.clone(),
            maturity_date: holding.maturity_date.clone(),
            current_price,
            current_price_cad,
            market_value_cad,
            cost_value_cad,
            gain_loss,
            gain_loss_percent,
            weight: 0.0,
            target_value: 0.0,
            target_delta_value: 0.0,
            target_delta_percent: 0.0,
            daily_change_percent: change_percent,
            fx_stale,
            price_is_stale,
        });
    }

    let total_target_weight: f64 = holdings.iter().map(|h| h.target_weight).sum();
    let mut target_cash_delta = 0.0f64;

    for hwp in &mut holdings_with_price {
        hwp.weight = if total_value != 0.0 {
            (hwp.market_value_cad / total_value) * 100.0
        } else {
            0.0
        };
        hwp.target_value = total_value * (hwp.target_weight / 100.0);
        hwp.target_delta_value = hwp.target_value - hwp.market_value_cad;
        hwp.target_delta_percent = hwp.target_weight - hwp.weight;

        if hwp.asset_type.as_str() == "cash" {
            target_cash_delta += hwp.market_value_cad - hwp.target_value;
        }
    }

    let total_gain_loss = total_value - total_cost;
    let total_gain_loss_percent = if total_cost != 0.0 {
        (total_gain_loss / total_cost) * 100.0
    } else {
        0.0
    };

    PortfolioSnapshot {
        holdings: holdings_with_price,
        total_value,
        total_cost,
        total_gain_loss,
        total_gain_loss_percent,
        daily_pnl,
        last_updated,
        base_currency: base_currency.to_string(),
        total_target_weight,
        target_cash_delta,
        realized_gains,
        annual_dividend_income,
        requires_cost_basis_selection: false,
    }
}
