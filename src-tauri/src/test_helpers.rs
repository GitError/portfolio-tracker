#![cfg(test)]

use crate::types::{AccountType, AssetType, Holding, HoldingId};

/// Canonical `Holding` builder for unit tests. Previously duplicated across
/// `portfolio.rs`, `csv.rs`, and `commands/mod.rs` (#605).
pub(crate) fn make_holding(
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
