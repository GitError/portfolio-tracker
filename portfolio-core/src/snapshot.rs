use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::fx::convert_to_base;
use crate::types::{FxRate, Holding, HoldingWithPrice, PortfolioSnapshot, PriceData};

/// Price is considered stale after 24 h — covers overnight/weekend gaps when markets are closed.
const PRICE_STALE_SECS: i64 = 24 * 3600;

/// Returns true if `updated_at` (an RFC 3339 timestamp) is older than `PRICE_STALE_SECS`,
/// or unparseable.
fn is_price_stale(updated_at: &str) -> bool {
    DateTime::parse_from_rfc3339(updated_at)
        .ok()
        .map(|t| {
            Utc::now()
                .signed_duration_since(t.with_timezone(&Utc))
                .num_seconds()
                > PRICE_STALE_SECS
        })
        .unwrap_or(true) // if timestamp unparseable, treat as stale
}

/// Build a `PortfolioSnapshot` from raw holdings, cached prices, and FX rates.
///
/// All monetary values in the snapshot are expressed in `base_currency`.
/// `realized_gains` and `annual_dividend_income` are passed in from the caller
/// because they require separate DB queries.
///
/// Shared by `src-tauri` (desktop app) and `portfolio-mcp` (MCP server) so the
/// two never drift out of sync — see #615.
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
                Some(p) if p.price > 0.0 && p.price.is_finite() => {
                    let stale = is_price_stale(&p.updated_at);
                    (p.price, p.change_percent, stale)
                }
                // A cached price of exactly 0.0 is ambiguous: it may be a
                // genuinely worthless security (penny stock, delisted share,
                // warrant) or a bad API response. Trust it while fresh; once
                // stale (> 24h old), it's far more likely bad data, so fall
                // back to cost_basis the same as a cache miss.
                Some(p) if p.price == 0.0 => {
                    let stale = is_price_stale(&p.updated_at);
                    if stale {
                        (holding.cost_basis, 0.0, true)
                    } else {
                        (0.0, p.change_percent, false)
                    }
                }
                // Negative, infinite, or NaN prices are never legitimate — treat as a cache miss.
                _ => (holding.cost_basis, 0.0, true),
            }
        };

        let (fx_rate, fx_stale) = if holding.currency.eq_ignore_ascii_case(base_currency) {
            (1.0, false)
        } else {
            // `convert_to_base` is the single source of truth for FX lookups:
            // it already guards against zero and non-finite rates on both the
            // direct and inverted pair (see #633). Do not short-circuit it
            // with a raw fx_map lookup — that bypasses the guard entirely.
            match convert_to_base(1.0, &holding.currency, base_currency, cached_fx) {
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

        // Compute daily PnL contribution for this holding.
        // Use a consistent UTC date boundary to avoid off-by-one errors at midnight.
        let today_utc = Utc::now().date_naive().to_string(); // "YYYY-MM-DD"
        let mut price_is_stale = price_is_stale;
        let created_date_utc = match holding.created_at.get(..10) {
            Some(d) => d,
            None => {
                tracing::warn!(
                    holding_id = %holding.id,
                    created_at = %holding.created_at,
                    "Corrupted created_at; holding excluded from daily PnL"
                );
                price_is_stale = true;
                ""
            }
        };
        if !created_date_utc.is_empty() && created_date_utc < today_utc.as_str() {
            // Prior-day holding: use Yahoo's day-over-day change_percent against current market value.
            daily_pnl += market_value_cad * (change_percent / 100.0);
        } else if !created_date_utc.is_empty() && created_date_utc == today_utc.as_str() {
            // Same-day purchase: no prior-day close available from the price feed.
            // Use cost_basis per unit as the prior-close proxy so the Dashboard
            // reflects the actual gain since purchase rather than showing $0.
            // daily_pnl_holding = (current_price - cost_basis_per_unit) * quantity * fx_rate
            let cost_per_unit = holding.cost_basis; // cost_basis is already per-unit
            daily_pnl += (current_price - cost_per_unit) * holding.quantity * fx_rate;
        }

        holdings_with_price.push(HoldingWithPrice {
            holding: holding.clone(),
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

    let total_target_weight: f64 = holdings
        .iter()
        .filter_map(|holding| holding.target_weight)
        .sum();
    let mut target_cash_delta = 0.0f64;

    for holding in &mut holdings_with_price {
        holding.weight = if total_value != 0.0 {
            (holding.market_value_cad / total_value) * 100.0
        } else {
            0.0
        };
        let effective_target_weight = holding.target_weight.unwrap_or(0.0);
        holding.target_value = total_value * (effective_target_weight / 100.0);
        holding.target_delta_value = holding.target_value - holding.market_value_cad;
        holding.target_delta_percent = effective_target_weight - holding.weight;

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
        // Callers set this after calling build_portfolio_snapshot because
        // config access is not available here. Defaults to false.
        requires_cost_basis_selection: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AccountType, AssetType, FxRate, HoldingId, PriceData};
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
            target_weight: None,
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
        holdings[0].target_weight = Some(60.0);
        holdings[1].target_weight = Some(10.0);

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
    fn build_portfolio_snapshot_same_day_purchase_uses_cost_basis_for_daily_pnl() {
        // A holding created today has no prior-day close from the price feed.
        // daily_pnl should reflect (current_price - cost_basis) * quantity, not 0.
        let today = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let mut holding = make_holding("AAPL", AssetType::Stock, 10.0, 100.0, "CAD");
        holding.created_at = today;

        let prices = vec![PriceData {
            symbol: "AAPL".to_string(),
            price: 120.0,
            currency: "CAD".to_string(),
            change: 2.0,
            change_percent: 5.0, // day-over-day pct — should NOT be used for same-day purchases
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

        // Expected: (120 - 100) * 10 = 200 (gain since purchase used as daily proxy)
        // NOT 0 (old behaviour) and NOT 60 (market_value * change_percent / 100)
        assert!(
            (snapshot.daily_pnl - 200.0).abs() < 0.001,
            "expected daily_pnl == 200 for same-day purchase using cost-basis proxy, got {}",
            snapshot.daily_pnl
        );
    }

    #[test]
    fn build_portfolio_snapshot_same_day_purchase_zero_gain_when_at_cost() {
        // Same-day purchase where current price == cost basis → daily_pnl == 0.
        let today = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let mut holding = make_holding("FLAT", AssetType::Stock, 10.0, 100.0, "CAD");
        holding.created_at = today;

        let prices = vec![PriceData {
            symbol: "FLAT".to_string(),
            price: 100.0, // same as cost_basis
            currency: "CAD".to_string(),
            change: 0.0,
            change_percent: 1.0, // irrelevant — should be ignored
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

        assert!(
            snapshot.daily_pnl.abs() < 0.001,
            "expected daily_pnl == 0 when same-day price equals cost basis, got {}",
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
    fn build_portfolio_snapshot_stale_but_present_price_is_used_over_cost_basis() {
        // Regression guard for #316/#577: a cached price older than 60 minutes
        // (market closed, no refresh) must still be used as current_price —
        // cost_basis is only a fallback for a true cache miss (no row at all).
        let holding = make_holding("AAPL", AssetType::Stock, 10.0, 50.0, "USD");
        let three_hours_ago = (Utc::now() - chrono::Duration::hours(3)).to_rfc3339();
        let prices = vec![PriceData {
            symbol: "AAPL".to_string(),
            price: 180.0,
            currency: "USD".to_string(),
            change: 0.0,
            change_percent: 0.0,
            updated_at: three_hours_ago,
            open: None,
            previous_close: None,
            volume: None,
        }];

        let snapshot = build_portfolio_snapshot(
            &[holding],
            &prices,
            &[],
            "USD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        // Stale (3h old) cached price is used, not cost_basis (50.0).
        assert!((snapshot.holdings[0].current_price - 180.0).abs() < 0.001);
        // Still under 24h, so not flagged as stale.
        assert!(!snapshot.holdings[0].price_is_stale);
    }

    #[test]
    fn build_portfolio_snapshot_price_older_than_24h_flagged_stale_but_still_used() {
        // A price older than PRICE_STALE_SECS (24h) is flagged stale for the UI,
        // but must still be used as current_price rather than cost_basis.
        let holding = make_holding("AAPL", AssetType::Stock, 10.0, 50.0, "USD");
        let two_days_ago = (Utc::now() - chrono::Duration::hours(48)).to_rfc3339();
        let prices = vec![PriceData {
            symbol: "AAPL".to_string(),
            price: 180.0,
            currency: "USD".to_string(),
            change: 0.0,
            change_percent: 0.0,
            updated_at: two_days_ago,
            open: None,
            previous_close: None,
            volume: None,
        }];

        let snapshot = build_portfolio_snapshot(
            &[holding],
            &prices,
            &[],
            "USD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        assert!((snapshot.holdings[0].current_price - 180.0).abs() < 0.001);
        assert!(snapshot.holdings[0].price_is_stale);
    }

    #[test]
    fn build_portfolio_snapshot_fresh_zero_price_is_treated_as_legitimate() {
        // Regression guard for #618: a fresh cached price of exactly 0.0 may be
        // a genuinely worthless security (penny stock, delisted, warrant), not
        // just a bad API response. While fresh (< 24h old), it must be trusted
        // and used as-is rather than silently replaced with cost_basis.
        let holding = make_holding("WORTHLESS", AssetType::Stock, 10.0, 50.0, "USD");
        let prices = vec![PriceData {
            symbol: "WORTHLESS".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
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
            "USD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        assert!((snapshot.holdings[0].current_price - 0.0).abs() < 0.001);
        assert!((snapshot.holdings[0].market_value_cad - 0.0).abs() < 0.001);
        assert!(!snapshot.holdings[0].price_is_stale);
    }

    #[test]
    fn build_portfolio_snapshot_stale_zero_price_falls_back_to_cost_basis() {
        // Regression guard for #609/#618: a cached price of 0.0 that is also
        // stale (> 24h old) is almost certainly a bad API response rather than
        // a real price — fall back to cost_basis rather than using 0.0, which
        // would otherwise wipe out market_value_cad and produce a bogus -100%
        // gain_loss_percent.
        let holding = make_holding("BADPRICE", AssetType::Stock, 10.0, 50.0, "USD");
        let two_days_ago = (Utc::now() - chrono::Duration::hours(48)).to_rfc3339();
        let prices = vec![PriceData {
            symbol: "BADPRICE".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            change: 0.0,
            change_percent: 0.0,
            updated_at: two_days_ago,
            open: None,
            previous_close: None,
            volume: None,
        }];

        let snapshot = build_portfolio_snapshot(
            &[holding],
            &prices,
            &[],
            "USD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        assert!((snapshot.holdings[0].current_price - 50.0).abs() < 0.001);
        assert!((snapshot.holdings[0].market_value_cad - 500.0).abs() < 0.001);
        assert!((snapshot.holdings[0].gain_loss).abs() < 0.001);
        assert!(snapshot.holdings[0].price_is_stale);
    }

    #[test]
    fn build_portfolio_snapshot_cached_negative_price_falls_back_to_cost_basis() {
        // Regression guard for #609: negative cached prices are equally invalid.
        let holding = make_holding("BADPRICE", AssetType::Stock, 10.0, 50.0, "USD");
        let prices = vec![PriceData {
            symbol: "BADPRICE".to_string(),
            price: -5.0,
            currency: "USD".to_string(),
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
            "USD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        assert!((snapshot.holdings[0].current_price - 50.0).abs() < 0.001);
        assert!(snapshot.holdings[0].price_is_stale);
    }

    #[test]
    fn build_portfolio_snapshot_direct_zero_fx_rate_marks_stale_instead_of_zeroing_value() {
        // Regression guard for #633: a cached direct-pair FX rate of exactly 0.0
        // must be treated as unavailable (fx_stale = true), not multiplied in —
        // convert_to_base already guards this, but the fast-path fx_map lookup
        // in build_portfolio_snapshot must not bypass that guard.
        let holding = make_holding("AAPL", AssetType::Stock, 10.0, 100.0, "USD");
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
        // Direct pair USDCAD cached at exactly 0.0.
        let fx = vec![FxRate {
            pair: "USDCAD".to_string(),
            rate: 0.0,
            updated_at: Utc::now().to_rfc3339(),
        }];

        let snapshot = build_portfolio_snapshot(
            &[holding],
            &prices,
            &fx,
            "CAD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        assert!(
            snapshot.holdings[0].fx_stale,
            "a zero direct-pair FX rate must be treated as unavailable, not used as-is"
        );
        // market_value_cad must NOT be zeroed out — fx_rate should default to
        // 1.0 (source-currency passthrough) when the cached rate is invalid.
        assert!(
            (snapshot.holdings[0].market_value_cad - 1500.0).abs() < 0.001,
            "market_value_cad should fall back to source-currency value (1500), got {}",
            snapshot.holdings[0].market_value_cad
        );
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
    fn build_portfolio_snapshot_infinite_price_falls_back_to_cost_basis() {
        // Regression guard for #638: an infinite cached price must not
        // propagate into market_value_cad — Infinity > 0.0 is true, so the
        // existing `> 0.0` check alone accepts it. Must fall back to cost_basis.
        let holding = make_holding("BADFEED", AssetType::Stock, 10.0, 50.0, "CAD");
        let prices = vec![PriceData {
            symbol: "BADFEED".to_string(),
            price: f64::INFINITY,
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

        assert!(
            (snapshot.holdings[0].current_price - 50.0).abs() < 0.001,
            "infinite price should fall back to cost_basis, got {}",
            snapshot.holdings[0].current_price
        );
        assert!(snapshot.holdings[0].market_value_cad.is_finite());
        assert!(snapshot.holdings[0].price_is_stale);
    }

    #[test]
    fn build_portfolio_snapshot_nan_price_falls_back_to_cost_basis() {
        // Regression guard for #638: NaN must also be rejected explicitly.
        let holding = make_holding("BADFEED2", AssetType::Stock, 10.0, 50.0, "CAD");
        let prices = vec![PriceData {
            symbol: "BADFEED2".to_string(),
            price: f64::NAN,
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

        assert!(
            (snapshot.holdings[0].current_price - 50.0).abs() < 0.001,
            "NaN price should fall back to cost_basis, got {}",
            snapshot.holdings[0].current_price
        );
        assert!(snapshot.holdings[0].market_value_cad.is_finite());
        assert!(snapshot.holdings[0].price_is_stale);
    }

    #[test]
    fn build_portfolio_snapshot_infinite_fx_rate_marks_stale() {
        // Regression guard for #638: an infinite cached FX rate must not
        // propagate into market_value_cad.
        let holding = make_holding("AAPL", AssetType::Stock, 10.0, 100.0, "USD");
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
        let fx = vec![FxRate {
            pair: "USDCAD".to_string(),
            rate: f64::INFINITY,
            updated_at: Utc::now().to_rfc3339(),
        }];

        let snapshot = build_portfolio_snapshot(
            &[holding],
            &prices,
            &fx,
            "CAD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
            0.0,
        );

        assert!(
            snapshot.holdings[0].fx_stale,
            "an infinite FX rate must be treated as unavailable"
        );
        assert!(snapshot.holdings[0].market_value_cad.is_finite());
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
