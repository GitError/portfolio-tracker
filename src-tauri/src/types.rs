use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

// ── ID newtypes ───────────────────────────────────────────────────────────────

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
impl TS for HoldingId {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &ts_rs::Config) -> String { "HoldingId".to_string() }
    fn inline(_: &ts_rs::Config) -> String { "string".to_string() }
    fn decl(_: &ts_rs::Config) -> String { "type HoldingId = string;".to_string() }
    fn decl_concrete(_: &ts_rs::Config) -> String { "type HoldingId = string;".to_string() }
    fn visit_dependencies(_: &mut impl ts_rs::TypeVisitor) {}
    fn visit_generics(_: &mut impl ts_rs::TypeVisitor) {}
    fn output_path() -> Option<std::path::PathBuf> {
        Some(std::path::PathBuf::from("HoldingId.ts"))
    }
}

/// Typed wrapper for a price-alert's UUID string.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct AlertId(pub String);

impl std::fmt::Display for AlertId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl TS for AlertId {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &ts_rs::Config) -> String { "AlertId".to_string() }
    fn inline(_: &ts_rs::Config) -> String { "string".to_string() }
    fn decl(_: &ts_rs::Config) -> String { "type AlertId = string;".to_string() }
    fn decl_concrete(_: &ts_rs::Config) -> String { "type AlertId = string;".to_string() }
    fn visit_dependencies(_: &mut impl ts_rs::TypeVisitor) {}
    fn visit_generics(_: &mut impl ts_rs::TypeVisitor) {}
    fn output_path() -> Option<std::path::PathBuf> {
        Some(std::path::PathBuf::from("AlertId.ts"))
    }
}

/// Typed wrapper for a transaction's UUID string.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct TransactionId(pub String);

impl std::fmt::Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl TS for TransactionId {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &ts_rs::Config) -> String { "TransactionId".to_string() }
    fn inline(_: &ts_rs::Config) -> String { "string".to_string() }
    fn decl(_: &ts_rs::Config) -> String { "type TransactionId = string;".to_string() }
    fn decl_concrete(_: &ts_rs::Config) -> String { "type TransactionId = string;".to_string() }
    fn visit_dependencies(_: &mut impl ts_rs::TypeVisitor) {}
    fn visit_generics(_: &mut impl ts_rs::TypeVisitor) {}
    fn output_path() -> Option<std::path::PathBuf> {
        Some(std::path::PathBuf::from("TransactionId.ts"))
    }
}

// ── Transaction types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
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
            other => Err(format!("Unknown transaction type: {}", other)),
        }
    }
}

/// A single buy or sell transaction for a holding.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub id: TransactionId,
    pub holding_id: HoldingId,
    /// "buy" | "sell"
    pub transaction_type: TransactionType,
    pub quantity: f64,
    /// Price per unit in the holding's original currency.
    pub price: f64,
    /// ISO 8601 timestamp of when the transaction occurred.
    pub transacted_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TransactionInput {
    pub holding_id: HoldingId,
    pub transaction_type: TransactionType,
    pub quantity: f64,
    pub price: f64,
    pub transacted_at: String,
}

// ── Realized gains types ──────────────────────────────────────────────────────

/// One matched lot that was sold.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RealizedLot {
    /// ISO date of the sell transaction (YYYY-MM-DD).
    pub sold_at: String,
    pub quantity: f64,
    /// quantity × sell_price
    pub proceeds: f64,
    /// quantity × cost_per_unit (method-dependent)
    pub cost_basis: f64,
    /// proceeds − cost_basis
    pub gain_loss: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RealizedGainsSummary {
    pub total_realized_gain: f64,
    pub total_proceeds: f64,
    pub total_cost_basis: f64,
    pub lots: Vec<RealizedLot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub name: String,
    pub account_type: String,
    pub institution: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountRequest {
    pub name: String,
    pub account_type: String,
    pub institution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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
    pub target_weight: f64,
    pub created_at: String,
    pub updated_at: String,
    pub indicated_annual_dividend: Option<f64>,
    pub indicated_annual_dividend_currency: Option<String>,
    pub dividend_frequency: Option<String>,
    pub maturity_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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
    pub target_weight: f64,
    pub indicated_annual_dividend: Option<f64>,
    pub indicated_annual_dividend_currency: Option<String>,
    pub dividend_frequency: Option<String>,
    pub maturity_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct FxRate {
    pub pair: String,
    pub rate: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct HoldingWithPrice {
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
    pub target_weight: f64,
    pub created_at: String,
    pub updated_at: String,
    pub indicated_annual_dividend: Option<f64>,
    pub indicated_annual_dividend_currency: Option<String>,
    pub dividend_frequency: Option<String>,
    pub maturity_date: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StressScenario {
    pub name: String,
    pub shocks: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SymbolResult {
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub exchange: String,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ImportError {
    pub row: usize,
    pub symbol: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported: Vec<Holding>,
    pub skipped: Vec<ImportError>,
    pub total_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StressResult {
    pub scenario: String,
    pub current_value: f64,
    pub stressed_value: f64,
    pub total_impact: f64,
    pub total_impact_percent: f64,
    pub holding_breakdown: Vec<StressHoldingResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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
    pub target_weight: f64,
    /// "ready" | "cash" | "duplicate" | "invalid_symbol" | "validation_failed"
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PreviewImportResult {
    pub rows: Vec<PreviewRow>,
    pub ready_count: usize,
    pub skip_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PerformancePoint {
    pub date: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Dividend {
    pub id: i64,
    pub holding_id: HoldingId,
    pub symbol: String,
    pub amount_per_unit: f64,
    pub currency: String,
    pub ex_date: String,
    pub pay_date: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct DividendInput {
    pub holding_id: HoldingId,
    pub amount_per_unit: f64,
    pub currency: String,
    pub ex_date: String,
    pub pay_date: String,
}

/// Direction for a price alert threshold.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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
            other => Err(format!("Unknown alert direction: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PriceAlertInput {
    pub symbol: String,
    pub direction: AlertDirection,
    pub threshold: f64,
    pub currency: String,
    pub note: String,
}

/// Returned by the `refresh_prices` command.
/// Separates successfully refreshed prices from symbols that failed.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResult {
    pub prices: Vec<PriceData>,
    /// Symbols for which the price fetch failed (network error, HTTP error, parse failure).
    pub failed_symbols: Vec<String>,
    /// IDs of price alerts that were triggered during this refresh.
    pub triggered_alerts: Vec<String>,
    /// Human-readable errors that occurred while evaluating price alerts.
    /// Non-empty when one or more alert checks failed so the frontend can surface them.
    pub alert_errors: Vec<String>,
    /// Error message if the portfolio snapshot could not be recorded after the refresh.
    /// The refresh itself succeeded — this only indicates the performance history entry failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_error: Option<String>,
}

/// Full data export payload — includes all user data for backup/restore.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ExportPayload {
    pub holdings: Vec<Holding>,
    pub alerts: Vec<PriceAlert>,
    pub config: Vec<(String, String)>,
    #[serde(default)]
    pub transactions: Vec<Transaction>,
    #[serde(default)]
    pub dividends: Vec<Dividend>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SymbolMetadata {
    pub symbol: String,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub country: Option<String>,
    pub market_cap: Option<f64>,
    pub pe_ratio: Option<f64>,
    pub dividend_yield: Option<f64>,
    pub beta: Option<f64>,
    pub eps: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SectorWeight {
    pub sector: String,
    pub weight_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CountryWeight {
    pub country: String,
    pub weight_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioRiskMetrics {
    pub weighted_beta: Option<f64>,
    pub portfolio_yield: f64,
    pub largest_position_weight: f64,
    pub top_sector: Option<String>,
    pub concentration_hhi: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioAnalytics {
    pub metadata: Vec<SymbolMetadata>,
    pub risk_metrics: PortfolioRiskMetrics,
    pub sector_breakdown: Vec<SectorWeight>,
    pub country_breakdown: Vec<CountryWeight>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RebalanceSuggestion {
    pub holding_id: HoldingId,
    pub symbol: String,
    pub name: String,
    pub current_value_cad: f64,
    pub target_value_cad: f64,
    pub current_weight: f64,      // actual % of portfolio
    pub target_weight: f64,       // user-set target %
    pub drift: f64,               // current_weight - target_weight (percentage points)
    pub suggested_trade_cad: f64, // positive = sell, negative = buy
    pub suggested_units: f64,     // positive = sell, negative = buy
    pub current_price_cad: f64,
}

// ── Pagination ────────────────────────────────────────────────────────────────

/// Generic paginated response wrapper for any list type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}
