use serde::{Deserialize, Serialize};

// Canonical portfolio types shared with `src-tauri` — see `portfolio-core`
// and #615. Re-exported here so every other module in this crate can keep
// using `crate::types::X` unchanged.
pub use portfolio_core::types::{
    AccountType, AssetType, FxRate, Holding, HoldingId, PortfolioSnapshot, PriceData,
    StressResult, StressScenario,
};

// ── ID newtypes ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct AlertId(pub String);

impl std::fmt::Display for AlertId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct TransactionId(pub String);

impl std::fmt::Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Buy,
    Sell,
}

impl TransactionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionType::Buy => "buy",
            TransactionType::Sell => "sell",
        }
    }
}

impl std::str::FromStr for TransactionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "buy" => Ok(TransactionType::Buy),
            "sell" => Ok(TransactionType::Sell),
            other => Err(format!("Unknown transaction type: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AlertDirection {
    Above,
    Below,
}

impl AlertDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertDirection::Above => "above",
            AlertDirection::Below => "below",
        }
    }
}

impl std::str::FromStr for AlertDirection {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "above" => Ok(AlertDirection::Above),
            "below" => Ok(AlertDirection::Below),
            other => Err(format!("Unknown alert direction: {other}")),
        }
    }
}

// ── Core data types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldingInput {
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub account: AccountType,
    pub account_id: Option<String>,
    pub quantity: f64,
    pub cost_basis: f64,
    pub currency: String,
    pub exchange: String,
    pub target_weight: Option<f64>,
    pub indicated_annual_dividend: Option<f64>,
    pub indicated_annual_dividend_currency: Option<String>,
    pub dividend_frequency: Option<String>,
    pub maturity_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub id: TransactionId,
    pub holding_id: HoldingId,
    pub transaction_type: TransactionType,
    pub quantity: f64,
    pub price: f64,
    pub transacted_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionInput {
    pub holding_id: HoldingId,
    pub transaction_type: TransactionType,
    pub quantity: f64,
    pub price: f64,
    pub transacted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceAlert {
    pub id: AlertId,
    pub symbol: String,
    pub direction: AlertDirection,
    pub threshold: f64,
    pub currency: String,
    pub note: String,
    pub triggered: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceAlertInput {
    pub symbol: String,
    pub direction: AlertDirection,
    pub threshold: f64,
    pub currency: String,
    pub note: String,
}

/// Typed wrapper for a dividend's UUID string.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct DividendId(pub String);

impl std::fmt::Display for DividendId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// ── Accounts / Dividends ─────────────────────────────────────────────────────
// Not part of `portfolio-core` (unlike Holding/PortfolioSnapshot) since they
// aren't consumed by `build_portfolio_snapshot`; mirrored from
// `src-tauri/src/types.rs` instead.

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub name: String,
    pub account_type: String,
    pub institution: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dividend {
    pub id: DividendId,
    pub holding_id: HoldingId,
    pub symbol: String,
    pub amount_per_unit: f64,
    pub currency: String,
    pub ex_date: String,
    pub pay_date: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DividendInput {
    pub holding_id: HoldingId,
    pub amount_per_unit: f64,
    pub currency: String,
    pub ex_date: String,
    pub pay_date: String,
}

