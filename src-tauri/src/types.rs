use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    Tfsa,
    Rrsp,
    Taxable,
    Cash,
}

impl AccountType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountType::Tfsa => "tfsa",
            AccountType::Rrsp => "rrsp",
            AccountType::Taxable => "taxable",
            AccountType::Cash => "cash",
        }
    }
}

impl std::str::FromStr for AccountType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tfsa" => Ok(AccountType::Tfsa),
            "rrsp" => Ok(AccountType::Rrsp),
            "taxable" => Ok(AccountType::Taxable),
            "cash" => Ok(AccountType::Cash),
            other => Err(format!("Unknown account type: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Holding {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub account: AccountType,
    pub quantity: f64,
    pub cost_basis: f64,
    pub currency: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldingInput {
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub account: AccountType,
    pub quantity: f64,
    pub cost_basis: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceData {
    pub symbol: String,
    pub price: f64,
    pub currency: String,
    pub change: f64,
    pub change_percent: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxRate {
    pub pair: String,
    pub rate: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldingWithPrice {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub account: AccountType,
    pub quantity: f64,
    pub cost_basis: f64,
    pub currency: String,
    pub created_at: String,
    pub updated_at: String,
    pub current_price: f64,
    pub current_price_cad: f64,
    pub market_value_cad: f64,
    pub cost_value_cad: f64,
    pub gain_loss: f64,
    pub gain_loss_percent: f64,
    pub weight: f64,
    pub daily_change_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StressScenario {
    pub name: String,
    pub shocks: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StressHoldingResult {
    pub holding_id: String,
    pub symbol: String,
    pub name: String,
    pub current_value: f64,
    pub stressed_value: f64,
    pub impact: f64,
    pub shock_applied: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolResult {
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub exchange: String,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportError {
    pub row: usize,
    pub symbol: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported: Vec<Holding>,
    pub skipped: Vec<ImportError>,
    pub total_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StressResult {
    pub scenario: String,
    pub current_value: f64,
    pub stressed_value: f64,
    pub total_impact: f64,
    pub total_impact_percent: f64,
    pub holding_breakdown: Vec<StressHoldingResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRow {
    pub row: usize,
    /// Symbol as written in the CSV (e.g. "BMO:CA")
    pub original_symbol: String,
    /// Resolved Yahoo Finance symbol (e.g. "BMO.TO"), empty when unresolvable
    pub resolved_symbol: String,
    pub name: String,
    pub asset_type: String,
    pub currency: String,
    pub exchange: String,
    pub quantity: f64,
    pub cost_basis: f64,
    /// "ready" | "cash" | "duplicate" | "invalid_symbol" | "validation_failed"
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewImportResult {
    pub rows: Vec<PreviewRow>,
    pub ready_count: usize,
    pub skip_count: usize,
}
