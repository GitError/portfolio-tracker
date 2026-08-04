use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// Canonical portfolio types shared with `portfolio-mcp` — see `portfolio-core`
// and #615. Re-exported here so every other module in this crate can keep
// using `crate::types::X` unchanged.
pub use portfolio_core::types::{
    AccountType, AssetType, FxRate, Holding, HoldingId, HoldingWithPrice, PortfolioSnapshot,
    PriceData, StressResult, StressScenario,
};

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
    fn name(_: &ts_rs::Config) -> String {
        "AlertId".to_string()
    }
    fn inline(_: &ts_rs::Config) -> String {
        "string".to_string()
    }
    fn decl(_: &ts_rs::Config) -> String {
        "type AlertId = string;".to_string()
    }
    fn decl_concrete(_: &ts_rs::Config) -> String {
        "type AlertId = string;".to_string()
    }
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
    fn name(_: &ts_rs::Config) -> String {
        "TransactionId".to_string()
    }
    fn inline(_: &ts_rs::Config) -> String {
        "string".to_string()
    }
    fn decl(_: &ts_rs::Config) -> String {
        "type TransactionId = string;".to_string()
    }
    fn decl_concrete(_: &ts_rs::Config) -> String {
        "type TransactionId = string;".to_string()
    }
    fn visit_dependencies(_: &mut impl ts_rs::TypeVisitor) {}
    fn visit_generics(_: &mut impl ts_rs::TypeVisitor) {}
    fn output_path() -> Option<std::path::PathBuf> {
        Some(std::path::PathBuf::from("TransactionId.ts"))
    }
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

impl TS for DividendId {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &ts_rs::Config) -> String {
        "DividendId".to_string()
    }
    fn inline(_: &ts_rs::Config) -> String {
        "string".to_string()
    }
    fn decl(_: &ts_rs::Config) -> String {
        "type DividendId = string;".to_string()
    }
    fn decl_concrete(_: &ts_rs::Config) -> String {
        "type DividendId = string;".to_string()
    }
    fn visit_dependencies(_: &mut impl ts_rs::TypeVisitor) {}
    fn visit_generics(_: &mut impl ts_rs::TypeVisitor) {}
    fn output_path() -> Option<std::path::PathBuf> {
        Some(std::path::PathBuf::from("DividendId.ts"))
    }
}

// ── Transaction types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
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
#[serde(rename_all = "camelCase")]
pub struct RealizedGainsSummary {
    pub total_realized_gain: f64,
    pub total_proceeds: f64,
    pub total_cost_basis: f64,
    pub lots: Vec<RealizedLot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub name: String,
    pub account_type: String,
    pub institution: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountRequest {
    pub name: String,
    pub account_type: String,
    pub institution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SymbolResult {
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub exchange: String,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportError {
    pub row: usize,
    pub symbol: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported: Vec<Holding>,
    pub skipped: Vec<ImportError>,
    pub total_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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
    pub target_weight: Option<f64>,
    /// "ready" | "cash" | "duplicate" | "invalid_symbol" | "validation_failed"
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PreviewImportResult {
    pub rows: Vec<PreviewRow>,
    pub ready_count: usize,
    pub skip_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PerformancePoint {
    pub date: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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
#[serde(rename_all = "camelCase")]
pub struct SectorWeight {
    pub sector: String,
    pub weight_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CountryWeight {
    pub country: String,
    pub weight_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioRiskMetrics {
    pub weighted_beta: Option<f64>,
    pub portfolio_yield: f64,
    pub largest_position_weight: f64,
    pub top_sector: Option<String>,
    pub concentration_hhi: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioAnalytics {
    pub metadata: Vec<SymbolMetadata>,
    pub risk_metrics: PortfolioRiskMetrics,
    pub sector_breakdown: Vec<SectorWeight>,
    pub country_breakdown: Vec<CountryWeight>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
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

// ── Import Plus Insights pipeline ──────────────────────────────────────────────
// See docs/superpowers/specs/2026-05-24-import-plus-insights-design.md.
// This is a new pipeline, separate from the legacy `parse_import_rows` path
// above (`ImportError`/`ImportResult`/`PreviewRow`/`PreviewImportResult`),
// which remains as a compatibility path per the design doc.

/// User-supplied context applied to every row of an import unless the file
/// provides stronger per-row account information.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportContext {
    /// TFSA, RRSP, FHSA, Taxable, Cash, Crypto, or Other.
    pub account_type: String,
    pub account_name: Option<String>,
    pub account_id: Option<String>,
    /// Hint such as "CanadianBankMultiSection", "GenericCSV".
    pub source_profile: Option<String>,
    /// User-selected column overrides: source column header -> canonical field name.
    #[serde(default)]
    pub column_overrides: HashMap<String, String>,
}

/// The action the import plan proposes for a given row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RowAction {
    Create,
    Update,
    Skip,
    NeedsFix,
    Warning,
}

/// Describes how a source column was mapped to a canonical holding field.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ColumnMapping {
    pub source_header: String,
    pub canonical_field: Option<String>,
    /// "exact" | "alias" | "profile" | "user" | "derived"
    pub confidence: String,
    pub reason: String,
}

/// Canonical interpretation of one source row, produced by the row normalizer.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedImportRow {
    pub row_number: usize,
    pub action: RowAction,
    pub symbol: Option<String>,
    pub resolved_symbol: Option<String>,
    pub name: Option<String>,
    pub asset_type: Option<String>,
    pub quantity: Option<f64>,
    pub cost_basis: Option<f64>,
    /// e.g. "average_cost", "derived:total_cost/qty"
    pub cost_basis_source: Option<String>,
    pub currency: Option<String>,
    pub book_value: Option<f64>,
    pub market_value: Option<f64>,
    pub exchange: Option<String>,
    pub target_weight: Option<f64>,
    pub account_type: String,
    pub account_name: Option<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    /// Original CSV values for this row, keyed by source header.
    pub raw: HashMap<String, String>,
    // ── Insight metadata: surfaced post-import, never written to the holdings table ──
    pub dividend_yield: Option<f64>,
    pub annualized_income: Option<f64>,
    pub ex_dividend_date: Option<String>,
}

/// Preview payload returned by `parse_import_file`. Never writes to the DB.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportPlan {
    pub profile_detected: String,
    pub column_mappings: Vec<ColumnMapping>,
    pub rows: Vec<NormalizedImportRow>,
    pub count_create: usize,
    pub count_update: usize,
    pub count_skip: usize,
    pub count_needs_fix: usize,
    pub count_warning: usize,
    pub suggested_account_type: Option<String>,
    pub suggested_account_number: Option<String>,
    pub cash_rows: Vec<NormalizedImportRow>,
}

/// Selected rows to write to the DB, as chosen by the user in the review UI.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportCommitRequest {
    pub plan_rows: Vec<NormalizedImportRow>,
    pub account_id: String,
    pub include_cash: bool,
}

/// Result of committing an import plan, including insight deltas for the
/// post-import summary panel.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportCommitResult {
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
    pub new_symbols: Vec<String>,
    pub changed_symbols: Vec<String>,
    /// Existing holdings in the target account not present in the imported file.
    /// Surfaced as review candidates only — never auto-deleted.
    pub missing_from_import: Vec<String>,
    pub stale_symbols: Vec<String>,
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
