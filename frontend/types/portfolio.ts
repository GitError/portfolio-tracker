// This file is the source of truth for TypeScript types.
// Types are validated against Rust bindings in frontend/types/bindings/.
// When Rust types change, run: npm run generate:types

export type AssetType = 'stock' | 'etf' | 'crypto' | 'cash';
export type AccountType = 'tfsa' | 'rrsp' | 'fhsa' | 'taxable' | 'crypto' | 'cash' | 'other';

export interface Account {
  id: string;
  name: string;
  accountType: AccountType;
  institution?: string;
  createdAt: string;
}

export interface CreateAccountRequest {
  name: string;
  accountType: AccountType;
  institution?: string | undefined;
}

export interface HoldingInput {
  symbol: string;
  name: string;
  assetType: AssetType;
  account: AccountType;
  accountId: string | null;
  quantity: number;
  costBasis: number;
  currency: string;
  exchange: string;
  targetWeight: number;
  indicatedAnnualDividend: number | null;
  indicatedAnnualDividendCurrency: string | null;
  dividendFrequency: 'monthly' | 'quarterly' | 'semi-annual' | 'annual' | 'irregular' | null;
  maturityDate: string | null;
}

export interface Holding {
  id: string;
  symbol: string;
  name: string;
  assetType: AssetType;
  account: AccountType;
  quantity: number;
  costBasis: number; // per unit, in original currency
  currency: string; // ISO currency code
  exchange: string; // exchange code, e.g. "NYSE", "TSX"
  targetWeight: number; // desired % of total portfolio value
  createdAt: string; // ISO 8601
  updatedAt: string; // ISO 8601
  indicatedAnnualDividend: number | null;
  indicatedAnnualDividendCurrency: string | null;
  dividendFrequency: 'monthly' | 'quarterly' | 'semi-annual' | 'annual' | 'irregular' | null;
  maturityDate: string | null;
}

export interface HoldingWithPrice extends Holding {
  currentPrice: number; // in original currency
  currentPriceCad: number; // converted to CAD
  marketValueCad: number; // quantity × currentPriceCad
  costValueCad: number; // quantity × costBasis × fxRate
  gainLoss: number; // marketValueCad - costValueCad
  gainLossPercent: number; // gainLoss / costValueCad
  weight: number; // marketValueCad / totalPortfolioValue
  targetValue: number; // desired value in base currency
  targetDeltaValue: number; // targetValue - marketValueCad
  targetDeltaPercent: number; // targetWeight - weight
  dailyChangePercent: number;
  /** True when the FX rate for this holding's currency was unavailable; values are shown in source currency. */
  fxStale: boolean;
  /** True when the cached price is older than 24 hours. Always false for cash holdings. */
  priceIsStale: boolean;
  // Inherited from Holding: indicatedAnnualDividend, indicatedAnnualDividendCurrency,
  // dividendFrequency, maturityDate
}

export interface PortfolioSnapshot {
  holdings: HoldingWithPrice[];
  totalValue: number; // sum of all marketValueCad
  totalCost: number; // sum of all costValueCad
  totalGainLoss: number;
  totalGainLossPercent: number;
  dailyPnl: number;
  lastUpdated: string; // ISO 8601
  /** The currency all values are expressed in. Defaults to "CAD". */
  baseCurrency: string;
  totalTargetWeight: number;
  targetCashDelta: number;
  /** Sum of all realized gains across holdings (AVCO by default). */
  realizedGains: number;
  /** Sum of (amountPerUnit × quantity) for dividends paid in the last 365 days. */
  annualDividendIncome: number;
  /** When true the user has never explicitly set a cost-basis method.
   * The app should prompt for a choice before displaying realized gains. */
  requiresCostBasisSelection?: boolean;
}

// ── Transaction types ──────────────────────────────────────────────────────────

export type TransactionType = 'buy' | 'sell';

export interface Transaction {
  id: string;
  holdingId: string;
  transactionType: TransactionType;
  quantity: number;
  price: number;
  transactedAt: string; // ISO 8601
  createdAt: string; // ISO 8601
}

export interface TransactionInput {
  holdingId: string;
  transactionType: TransactionType;
  quantity: number;
  price: number;
  transactedAt: string; // ISO 8601
}

// ── Realized gains types ──────────────────────────────────────────────────────

export interface RealizedLot {
  soldAt: string; // YYYY-MM-DD
  quantity: number;
  proceeds: number;
  costBasis: number;
  gainLoss: number;
}

export interface RealizedGainsSummary {
  totalRealizedGain: number;
  totalProceeds: number;
  totalCostBasis: number;
  lots: RealizedLot[];
}

export interface PriceData {
  symbol: string;
  price: number;
  currency: string;
  change: number;
  changePercent: number;
  updatedAt: string;
  open: number | null;
  previousClose: number | null;
  volume: number | null;
}

export interface RefreshResult {
  prices: PriceData[];
  /** Symbols for which the price fetch failed. Empty when all succeeded. */
  failedSymbols: string[];
  /** IDs of price alerts triggered during this refresh. */
  triggeredAlerts: string[];
  /** Error messages from alert evaluation that did not prevent the refresh. */
  alertErrors?: string[];
  /** Set when prices were updated but the portfolio snapshot could not be saved. */
  snapshotError?: string;
}

export interface ExportPayload {
  holdings: Holding[];
  alerts: PriceAlert[];
  config: [string, string][];
  transactions: Transaction[];
  dividends: Dividend[];
}

export interface FxRate {
  pair: string; // e.g. "USDCAD"
  rate: number;
  updatedAt: string;
}

export interface StressScenario {
  name: string;
  shocks: Record<string, number>; // keys: "stock"|"etf"|"crypto"|"fx_usd_cad"|"fx_cad_usd" etc, values: decimal (-0.10 = -10%)
}

export interface StressScenarioInfo extends StressScenario {
  description: string;
  historicalParallel: string;
}

export interface StressHoldingResult {
  holdingId: string;
  symbol: string;
  name: string;
  currentValue: number;
  stressedValue: number;
  impact: number;
  shockApplied: number;
}

export interface StressResult {
  scenario: string;
  currentValue: number;
  stressedValue: number;
  totalImpact: number;
  totalImpactPercent: number;
  holdingBreakdown: StressHoldingResult[];
}

export interface ImportError {
  row: number;
  symbol: string;
  reason: string;
}

export interface ImportResult {
  imported: Holding[];
  skipped: ImportError[];
  totalRows: number;
}

export interface SymbolResult {
  symbol: string;
  name: string;
  assetType: AssetType;
  exchange: string;
  currency: string;
}

export interface Dividend {
  id: number;
  holdingId: string;
  symbol: string;
  amountPerUnit: number;
  currency: string;
  exDate: string;
  payDate: string;
  createdAt: string;
}

export interface DividendInput {
  holdingId: string;
  amountPerUnit: number;
  currency: string;
  exDate: string;
  payDate: string;
}

export type AlertDirection = 'above' | 'below';

export interface PriceAlert {
  id: string;
  symbol: string;
  direction: AlertDirection;
  threshold: number;
  currency: string;
  note: string;
  triggered: boolean;
  createdAt: string;
}

export interface PriceAlertInput {
  symbol: string;
  direction: AlertDirection;
  threshold: number;
  currency: string;
  note: string;
}

export interface PreviewRow {
  row: number;
  originalSymbol: string;
  resolvedSymbol: string;
  name: string;
  assetType: string;
  currency: string;
  exchange: string;
  quantity: number;
  costBasis: number;
  targetWeight: number;
  status:
    | 'ready'
    | 'cash'
    | 'duplicate'
    | 'invalid_symbol'
    | 'validation_failed'
    | 'currency_mismatch';
  /** Present when status is 'currency_mismatch'. Format: '{actual}:{expected}', e.g. 'USD:CAD'. */
  currencyMismatchDetail?: string;
}

export interface PreviewImportResult {
  rows: PreviewRow[];
  readyCount: number;
  skipCount: number;
}
export interface RebalanceSuggestion {
  holdingId: string;
  symbol: string;
  name: string;
  currentValueCad: number;
  targetValueCad: number;
  currentWeight: number;
  targetWeight: number;
  drift: number;
  suggestedTradeCad: number;
  suggestedUnits: number;
  currentPriceCad: number;
}

export interface SymbolMetadata {
  symbol: string;
  sector?: string;
  industry?: string;
  country?: string;
  marketCap?: number;
  peRatio?: number;
  dividendYield?: number;
  beta?: number;
  eps?: number | null;
}

export interface SectorWeight {
  sector: string;
  weightPercent: number;
}

export interface CountryWeight {
  country: string;
  weightPercent: number;
}

export interface PortfolioRiskMetrics {
  weightedBeta?: number;
  portfolioYield: number;
  largestPositionWeight: number;
  topSector?: string;
  concentrationHhi: number;
}

export interface PortfolioAnalytics {
  metadata: SymbolMetadata[];
  riskMetrics: PortfolioRiskMetrics;
  sectorBreakdown: SectorWeight[];
  countryBreakdown: CountryWeight[];
}

export type InsightSeverity = 'info' | 'warning' | 'critical';

export type InsightDirection = 'buy' | 'sell' | 'review';

export interface ActionInsight {
  id: string;
  type:
    | 'target_drift'
    | 'concentration_risk'
    | 'idle_cash'
    | 'missing_targets'
    | 'account_imbalance';
  severity: InsightSeverity;
  /** Suggested action direction for grouping (buy/sell/review). */
  direction: InsightDirection;
  title: string;
  explanation: string;
  metrics?: Record<string, string | number>;
  action?: string;
  linkTo?: string; // route path like '/rebalance', '/holdings'
}

// ── Pagination ──

export interface PaginatedResult<T> {
  items: T[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

// ── Error types ──

export interface AppError {
  type: 'validation' | 'database' | 'network' | 'not_found' | 'conflict';
  message: string;
}

// ── Tauri Command Signatures ──

// invoke('get_portfolio')           → PortfolioSnapshot
// invoke('get_holdings')            → Holding[]
// invoke('add_holding', { holding }) → Holding        (omit id, createdAt, updatedAt)
// invoke('update_holding', { holding }) → Holding
// invoke('delete_holding', { id })  → boolean
// invoke('refresh_prices')          → RefreshResult
// invoke('get_performance', { range }) → { date: string; value: number }[]
// invoke('run_stress_test_cmd', { scenario }) → StressResult
// invoke('get_accounts')                       → Account[]
// invoke('add_account', { account })            → Account
// invoke('update_account', { id, account })     → Account
// invoke('delete_account', { id })              → boolean
// invoke('search_symbols', { query }) → SymbolResult[]
// invoke('get_symbol_price', { symbol }) → PriceData
// invoke('get_rebalance_suggestions', { driftThreshold }) → RebalanceSuggestion[]
// invoke('add_transaction', { tx }) → Transaction
// invoke('get_transactions', { holdingId? }) → Transaction[]
// invoke('delete_transaction', { id }) → boolean
// invoke('get_realized_gains', { holdingId? }) → RealizedGainsSummary
