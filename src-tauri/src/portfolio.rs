use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::fx::convert_to_base;
use crate::types::{FxRate, Holding, HoldingWithPrice, PortfolioSnapshot, PriceData};

/// Prices older than this are flagged as stale.
const PRICE_STALE_SECS: i64 = 24 * 3600;

/// Build a `PortfolioSnapshot` from raw holdings, cached prices, and FX rates.
///
/// All monetary values in the snapshot are expressed in `base_currency`.
/// `realized_gains` and `annual_dividend_income` are passed in from the caller
/// because they require separate DB queries.
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
                        .unwrap_or(true); // if timestamp unparseable, treat as stale
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
                        "FX rate unavailable — marking holding as fx_stale, values shown in source currency"
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

        // Exclude intraday purchases from daily PnL: a holding created today has
        // no prior-day close to compare against, so applying the day-over-day
        // change_percent would overstate the gain.
        // Use a consistent UTC date boundary to avoid off-by-one errors at midnight.
        let today_utc = Utc::now().date_naive().to_string(); // "YYYY-MM-DD"
        let created_date_utc = holding
            .created_at
            .get(..10)
            .and_then(|s| {
                // Only treat as a valid date if it parses; skip bad rows safely.
                if s.len() == 10 {
                    Some(s)
                } else {
                    None
                }
            })
            .unwrap_or("");
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

    let total_target_weight: f64 = holdings.iter().map(|holding| holding.target_weight).sum();
    let mut target_cash_delta = 0.0f64;

    for holding in &mut holdings_with_price {
        holding.weight = if total_value != 0.0 {
            (holding.market_value_cad / total_value) * 100.0
        } else {
            0.0
        };
        holding.target_value = total_value * (holding.target_weight / 100.0);
        holding.target_delta_value = holding.target_value - holding.market_value_cad;
        holding.target_delta_percent = holding.target_weight - holding.weight;

        if holding.asset_type.as_str() == "cash" {
            target_cash_delta += holding.market_value_cad - holding.target_value;
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
        // Callers (commands.rs) set this after calling build_portfolio_snapshot
        // because config access is not available here. Defaults to false.
        requires_cost_basis_selection: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AccountType, AssetType, FxRate, Holding, HoldingId, PriceData};
    use chrono::Utc;

    fn make_holding(
        symbol: &str,
        asset_type: AssetType,
        quantity: f64,
        cost_basis: f64,
        currency: &str,
    ) -> Holding {
        Holding {
            id: HoldingId(symbol.to_string()),
            symbol: symbol.to_string(),
            name: symbol.to_string(),
            asset_type,
            account: AccountType::Taxable,
            account_id: None,
            account_name: None,
            quantity,
            cost_basis,
            currency: currency.to_string(),
            exchange: String::new(),
            target_weight: 0.0,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            indicated_annual_dividend: None,
            indicated_annual_dividend_currency: None,
            dividend_frequency: None,
            maturity_date: None,
        }
    }

    #[test]
    fn build_portfolio_snapshot_converts_mixed_currency_holdings_into_base_currency() {
        let holdings = vec![
            make_holding("SHOP.TO", AssetType::Stock, 10.0, 100.0, "CAD"),
            make_holding("AAPL", AssetType::Stock, 5.0, 100.0, "USD"),
        ];
        let prices = vec![
            PriceData {
                symbol: "SHOP.TO".to_string(),
                price: 120.0,
                currency: "CAD".to_string(),
                change: 1.0,
                change_percent: 2.0,
                updated_at: Utc::now().to_rfc3339(),
                open: None,
                previous_close: None,
                volume: None,
            },
            PriceData {
                symbol: "AAPL".to_string(),
                price: 110.0,
                currency: "USD".to_string(),
                change: 1.0,
                change_percent: 10.0,
                updated_at: Utc::now().to_rfc3339(),
                open: None,
                previous_close: None,
                volume: None,
            },
        ];
        let fx = vec![FxRate {
            pair: "USDCAD".to_string(),
            rate: 1.25,
            updated_at: Utc::now().to_rfc3339(),
        }];

        let snapshot = build_portfolio_snapshot(
            &holdings,
            &prices,
            &fx,
            "CAD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        assert_eq!(snapshot.base_currency, "CAD");
        assert!((snapshot.holdings[0].market_value_cad - 1200.0).abs() < 0.001);
        assert!((snapshot.holdings[1].market_value_cad - 687.5).abs() < 0.001);
        assert!((snapshot.holdings[1].cost_value_cad - 625.0).abs() < 0.001);
        assert!((snapshot.total_value - 1887.5).abs() < 0.001);
        assert!((snapshot.total_cost - 1625.0).abs() < 0.001);
        assert!((snapshot.daily_pnl - 92.75).abs() < 0.001);
        assert_eq!(snapshot.total_target_weight, 0.0);
    }

    #[test]
    fn build_portfolio_snapshot_supports_non_cad_base_currency() {
        let holdings = vec![
            make_holding("RY.TO", AssetType::Stock, 2.0, 100.0, "CAD"),
            make_holding("MSFT", AssetType::Stock, 1.0, 200.0, "USD"),
        ];
        let prices = vec![
            PriceData {
                symbol: "RY.TO".to_string(),
                price: 110.0,
                currency: "CAD".to_string(),
                change: 0.0,
                change_percent: 0.0,
                updated_at: Utc::now().to_rfc3339(),
                open: None,
                previous_close: None,
                volume: None,
            },
            PriceData {
                symbol: "MSFT".to_string(),
                price: 220.0,
                currency: "USD".to_string(),
                change: 0.0,
                change_percent: 0.0,
                updated_at: Utc::now().to_rfc3339(),
                open: None,
                previous_close: None,
                volume: None,
            },
        ];
        let fx = vec![FxRate {
            pair: "CADUSD".to_string(),
            rate: 0.8,
            updated_at: Utc::now().to_rfc3339(),
        }];

        let snapshot = build_portfolio_snapshot(
            &holdings,
            &prices,
            &fx,
            "USD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        assert_eq!(snapshot.base_currency, "USD");
        assert!((snapshot.holdings[0].market_value_cad - 176.0).abs() < 0.001);
        assert!((snapshot.holdings[0].cost_value_cad - 160.0).abs() < 0.001);
        assert!((snapshot.holdings[1].market_value_cad - 220.0).abs() < 0.001);
        assert!((snapshot.total_value - 396.0).abs() < 0.001);
        assert!((snapshot.total_cost - 360.0).abs() < 0.001);
    }

    #[test]
    fn build_portfolio_snapshot_computes_target_deltas() {
        let mut holdings = vec![
            make_holding("AAPL", AssetType::Stock, 10.0, 100.0, "CAD"),
            make_holding("CAD-CASH", AssetType::Cash, 500.0, 1.0, "CAD"),
        ];
        holdings[0].target_weight = 60.0;
        holdings[1].target_weight = 10.0;

        let prices = vec![PriceData {
            symbol: "AAPL".to_string(),
            price: 120.0,
            currency: "CAD".to_string(),
            change: 0.0,
            change_percent: 0.0,
            updated_at: Utc::now().to_rfc3339(),
            open: None,
            previous_close: None,
            volume: None,
        }];

        let snapshot = build_portfolio_snapshot(
            &holdings,
            &prices,
            &[],
            "CAD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        assert!((snapshot.total_value - 1700.0).abs() < 0.001);
        assert!((snapshot.total_target_weight - 70.0).abs() < 0.001);
        assert!((snapshot.holdings[0].target_value - 1020.0).abs() < 0.001);
        assert!((snapshot.holdings[0].target_delta_value + 180.0).abs() < 0.001);
        assert!((snapshot.holdings[1].target_delta_value + 330.0).abs() < 0.001);
        assert!((snapshot.target_cash_delta - 330.0).abs() < 0.001);
    }

    #[test]
    fn build_portfolio_snapshot_excludes_intraday_purchase_from_daily_pnl() {
        // A holding created today should contribute 0 to daily_pnl.
        let today = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let mut holding = make_holding("AAPL", AssetType::Stock, 10.0, 100.0, "CAD");
        holding.created_at = today;

        let prices = vec![PriceData {
            symbol: "AAPL".to_string(),
            price: 120.0,
            currency: "CAD".to_string(),
            change: 2.0,
            change_percent: 5.0, // would be 60.0 CAD if applied
            updated_at: Utc::now().to_rfc3339(),
            open: None,
            previous_close: None,
            volume: None,
        }];

        let snapshot = build_portfolio_snapshot(
            &[holding],
            &prices,
            &[],
            "CAD",
            Utc::now().to_rfc3339(),
            0.0,
            0.0,
        );

        // market_value_cad = 10 * 120 = 1200; daily_pnl should be 0, not 60
        assert!(
            (snapshot.daily_pnl - 0.0).abs() < 0.001,
            "expected daily_pnl == 0 for intraday purchase, got {}",
            snapshot.daily_pnl
        );
    }

    #[test]
    fn build_portfolio_snapshot_includes_prior_day_holding_in_daily_pnl() {
        // A holding created yesterday (or earlier) should contribute normally.
        let yesterday = (Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let mut holding = make_holding("MSFT", AssetType::Stock, 10.0, 200.0, "CAD");
        holding.created_at = yesterday;

        let prices = vec![PriceData {
            symbol: "MSFT".to_string(),
            price: 220.0,
            currency: "CAD".to_string(),
            change: 20.0,
            change_percent: 10.0, // 10% of 2200 = 220
            updated_at: Utc::now().to_rfc3339(),
            open: None,
            previous_close: None,
            volume: None,
        }];

        let snapshot = build_portfolio_snapshot(
            &[holding],
            &prices,
            &[],
            "CAD",
            Utc::now().to_rfc3339(),
            0.0,
            0.0,
        );

        // market_value_cad = 10 * 220 = 2200; daily_pnl = 2200 * 0.10 = 220
        assert!(
            (snapshot.daily_pnl - 220.0).abs() < 0.001,
            "expected daily_pnl == 220 for prior-day holding, got {}",
            snapshot.daily_pnl
        );
    }

    #[test]
    fn build_portfolio_snapshot_empty_holdings_returns_zero_snapshot() {
        let snapshot = build_portfolio_snapshot(
            &[],
            &[],
            &[],
            "CAD",
            "2024-01-01T00:00:00Z".to_string(),
            50.0,
            10.0,
        );
        assert_eq!(snapshot.holdings.len(), 0);
        assert_eq!(snapshot.total_value, 0.0);
        assert_eq!(snapshot.total_cost, 0.0);
        assert_eq!(snapshot.total_gain_loss, 0.0);
        assert_eq!(snapshot.total_gain_loss_percent, 0.0);
        assert_eq!(snapshot.daily_pnl, 0.0);
        // realized_gains and annual_dividend_income are passed through
        assert!((snapshot.realized_gains - 50.0).abs() < 0.001);
        assert!((snapshot.annual_dividend_income - 10.0).abs() < 0.001);
    }

    #[test]
    fn build_portfolio_snapshot_cash_always_uses_price_1() {
        // Cash holdings must use price = 1.0 regardless of any cached price entry.
        let holding = make_holding("CAD-CASH", AssetType::Cash, 1000.0, 1.0, "CAD");
        // Provide a nonsense price entry — should be ignored for cash.
        let prices = vec![PriceData {
            symbol: "CAD-CASH".to_string(),
            price: 999.0,
            currency: "CAD".to_string(),
            change: 0.0,
            change_percent: 0.0,
            updated_at: Utc::now().to_rfc3339(),
            open: None,
            previous_close: None,
            volume: None,
        }];

        let snapshot = build_portfolio_snapshot(
            &[holding],
            &prices,
            &[],
            "CAD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        assert!((snapshot.holdings[0].current_price - 1.0).abs() < 0.001);
        assert!((snapshot.holdings[0].market_value_cad - 1000.0).abs() < 0.001);
    }

    #[test]
    fn build_portfolio_snapshot_missing_price_falls_back_to_cost_basis() {
        let holding = make_holding("UNKNOWN", AssetType::Stock, 10.0, 50.0, "CAD");

        let snapshot = build_portfolio_snapshot(
            &[holding],
            &[], // no prices
            &[],
            "CAD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        // With no price, current_price falls back to cost_basis (50.0)
        assert!((snapshot.holdings[0].current_price - 50.0).abs() < 0.001);
        assert!((snapshot.holdings[0].market_value_cad - 500.0).abs() < 0.001);
        // gain_loss should be zero when price == cost_basis
        assert!((snapshot.holdings[0].gain_loss).abs() < 0.001);
    }

    #[test]
    fn build_portfolio_snapshot_missing_fx_marks_holding_as_stale() {
        // When FX rate is unavailable, fx_stale = true and rate defaults to 1.0.
        let holding = make_holding("AAPL", AssetType::Stock, 1.0, 100.0, "USD");
        let prices = vec![PriceData {
            symbol: "AAPL".to_string(),
            price: 150.0,
            currency: "USD".to_string(),
            change: 0.0,
            change_percent: 0.0,
            updated_at: Utc::now().to_rfc3339(),
            open: None,
            previous_close: None,
            volume: None,
        }];

        // No FX rates provided — USDCAD unavailable
        let snapshot = build_portfolio_snapshot(
            &[holding],
            &prices,
            &[],
            "CAD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        assert!(
            snapshot.holdings[0].fx_stale,
            "fx_stale should be true when FX rate is unavailable"
        );
        // Rate defaults to 1.0, so market_value_cad == 150.0 (not converted)
        assert!((snapshot.holdings[0].market_value_cad - 150.0).abs() < 0.001);
    }

    #[test]
    fn build_portfolio_snapshot_weight_sums_to_100() {
        let holdings = vec![
            make_holding("A", AssetType::Stock, 1.0, 100.0, "CAD"),
            make_holding("B", AssetType::Stock, 1.0, 100.0, "CAD"),
            make_holding("C", AssetType::Stock, 1.0, 100.0, "CAD"),
        ];
        let prices = vec![
            PriceData {
                symbol: "A".to_string(),
                price: 100.0,
                currency: "CAD".to_string(),
                change: 0.0,
                change_percent: 0.0,
                updated_at: Utc::now().to_rfc3339(),
                open: None,
                previous_close: None,
                volume: None,
            },
            PriceData {
                symbol: "B".to_string(),
                price: 200.0,
                currency: "CAD".to_string(),
                change: 0.0,
                change_percent: 0.0,
                updated_at: Utc::now().to_rfc3339(),
                open: None,
                previous_close: None,
                volume: None,
            },
            PriceData {
                symbol: "C".to_string(),
                price: 300.0,
                currency: "CAD".to_string(),
                change: 0.0,
                change_percent: 0.0,
                updated_at: Utc::now().to_rfc3339(),
                open: None,
                previous_close: None,
                volume: None,
            },
        ];

        let snapshot = build_portfolio_snapshot(
            &holdings,
            &prices,
            &[],
            "CAD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        let total_weight: f64 = snapshot.holdings.iter().map(|h| h.weight).sum();
        assert!(
            (total_weight - 100.0).abs() < 0.001,
            "weights should sum to 100%, got {}",
            total_weight
        );
    }

    #[test]
    fn build_portfolio_snapshot_zero_cost_basis_gain_loss_percent_is_zero() {
        let holding = make_holding("FREE", AssetType::Stock, 10.0, 0.0, "CAD");
        let prices = vec![PriceData {
            symbol: "FREE".to_string(),
            price: 50.0,
            currency: "CAD".to_string(),
            change: 0.0,
            change_percent: 0.0,
            updated_at: Utc::now().to_rfc3339(),
            open: None,
            previous_close: None,
            volume: None,
        }];

        let snapshot = build_portfolio_snapshot(
            &[holding],
            &prices,
            &[],
            "CAD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        // Division by zero guard: gain_loss_percent should be 0.0 when cost == 0
        assert_eq!(snapshot.holdings[0].gain_loss_percent, 0.0);
    }
}
