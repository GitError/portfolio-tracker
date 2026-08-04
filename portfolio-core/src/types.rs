//! Canonical portfolio types shared by `src-tauri` and `portfolio-mcp`.
//!
//! These are the types that flow through [`crate::snapshot::build_portfolio_snapshot`].
//! Both crates re-export these directly (`src-tauri` enables the `ts` feature so
//! `ts-rs` bindings keep being generated from here).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Typed wrapper for a holding's UUID string.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct HoldingId(pub String);

impl std::fmt::Display for HoldingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// Manual TS impl avoids the ts-rs serde-compat proc-macro warning on #[serde(transparent)].
#[cfg(feature = "ts")]
impl ts_rs::TS for HoldingId {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &ts_rs::Config) -> String {
        "HoldingId".to_string()
    }
    fn inline(_: &ts_rs::Config) -> String {
        "string".to_string()
    }
    fn decl(_: &ts_rs::Config) -> String {
        "type HoldingId = string;".to_string()
    }
    fn decl_concrete(_: &ts_rs::Config) -> String {
        "type HoldingId = string;".to_string()
    }
    fn visit_dependencies(_: &mut impl ts_rs::TypeVisitor) {}
    fn visit_generics(_: &mut impl ts_rs::TypeVisitor) {}
    fn output_path() -> Option<std::path::PathBuf> {
        Some(std::path::PathBuf::from("HoldingId.ts"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum AssetType {
    Stock,
    Etf,
    Crypto,
    Cash,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetType::Stock => "stock",
            AssetType::Etf => "etf",
            AssetType::Crypto => "crypto",
            AssetType::Cash => "cash",
        }
    }
}

impl std::str::FromStr for AssetType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stock" => Ok(AssetType::Stock),
            "etf" => Ok(AssetType::Etf),
            "crypto" => Ok(AssetType::Crypto),
            "cash" => Ok(AssetType::Cash),
            other => Err(format!("Unknown asset type: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    Tfsa,
    Rrsp,
    Fhsa,
    Taxable,
    Crypto,
    Cash,
    Other,
}

impl AccountType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountType::Tfsa => "tfsa",
            AccountType::Rrsp => "rrsp",
            AccountType::Fhsa => "fhsa",
            AccountType::Taxable => "taxable",
            AccountType::Crypto => "crypto",
            AccountType::Cash => "cash",
            AccountType::Other => "other",
        }
    }
}

impl std::str::FromStr for AccountType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tfsa" => Ok(AccountType::Tfsa),
            "rrsp" => Ok(AccountType::Rrsp),
            "fhsa" => Ok(AccountType::Fhsa),
            "taxable" => Ok(AccountType::Taxable),
            "crypto" => Ok(AccountType::Crypto),
            "cash" => Ok(AccountType::Cash),
            "other" => Ok(AccountType::Other),
            other => Err(format!("Unknown account type: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Holding {
    pub id: HoldingId,
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub account: AccountType,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub quantity: f64,
    pub cost_basis: f64,
    pub currency: String,
    pub exchange: String,
    /// `None` means no target has been set; `Some(0.0)` means the user
    /// explicitly targeted this holding at 0% (a "sell everything" signal).
    pub target_weight: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
    pub indicated_annual_dividend: Option<f64>,
    pub indicated_annual_dividend_currency: Option<String>,
    pub dividend_frequency: Option<String>,
    pub maturity_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PriceData {
    pub symbol: String,
    pub price: f64,
    pub currency: String,
    pub change: f64,
    pub change_percent: f64,
    pub updated_at: String,
    pub open: Option<f64>,
    pub previous_close: Option<f64>,
    pub volume: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FxRate {
    pub pair: String,
    pub rate: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct HoldingWithPrice {
    #[serde(flatten)]
    pub holding: Holding,
    pub current_price: f64,
    pub current_price_cad: f64,
    pub market_value_cad: f64,
    pub cost_value_cad: f64,
    pub gain_loss: f64,
    pub gain_loss_percent: f64,
    pub weight: f64,
    pub target_value: f64,
    pub target_delta_value: f64,
    pub target_delta_percent: f64,
    pub daily_change_percent: f64,
    /// True when the FX rate for this holding's currency was not available;
    /// values are shown in the source currency as a fallback.
    pub fx_stale: bool,
    /// True when the cached price for this holding is older than the staleness
    /// threshold (currently 24 hours). Cash holdings are always false.
    pub price_is_stale: bool,
}

impl std::ops::Deref for HoldingWithPrice {
    type Target = Holding;
    fn deref(&self) -> &Holding {
        &self.holding
    }
}

impl std::ops::DerefMut for HoldingWithPrice {
    fn deref_mut(&mut self) -> &mut Holding {
        &mut self.holding
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PortfolioSnapshot {
    pub holdings: Vec<HoldingWithPrice>,
    pub total_value: f64,
    pub total_cost: f64,
    pub total_gain_loss: f64,
    pub total_gain_loss_percent: f64,
    pub daily_pnl: f64,
    pub last_updated: String,
    /// The currency all values are expressed in (user-configurable, default "CAD").
    pub base_currency: String,
    pub total_target_weight: f64,
    pub target_cash_delta: f64,
    /// Sum of realized gains across all holdings (AVCO method, all-time).
    pub realized_gains: f64,
    /// Sum of (amount_per_unit × quantity) for all dividends with a pay_date in the last 12 months.
    pub annual_dividend_income: f64,
    /// True when the user has never explicitly set a cost-basis method.
    /// The frontend should prompt the user to choose AVCO or FIFO before showing realized gains.
    #[serde(default)]
    pub requires_cost_basis_selection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct StressScenario {
    pub name: String,
    pub shocks: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct StressHoldingResult {
    pub holding_id: HoldingId,
    pub symbol: String,
    pub name: String,
    pub current_value: f64,
    pub stressed_value: f64,
    pub impact: f64,
    pub shock_applied: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct StressResult {
    pub scenario: String,
    pub current_value: f64,
    pub stressed_value: f64,
    pub total_impact: f64,
    pub total_impact_percent: f64,
    pub holding_breakdown: Vec<StressHoldingResult>,
}
