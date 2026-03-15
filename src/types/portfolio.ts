export type AssetType = 'stock' | 'etf' | 'crypto' | 'cash';
export type AccountType = 'tfsa' | 'rrsp' | 'taxable' | 'cash';

export interface HoldingInput {
  symbol: string;
  name: string;
  assetType: AssetType;
  account: AccountType;
  quantity: number;
  costBasis: number;
  currency: string;
  targetWeight: number;
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
  targetWeight: number; // desired % of total portfolio value
  createdAt: string; // ISO 8601
  updatedAt: string; // ISO 8601
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
}

export interface PriceData {
  symbol: string;
  price: number;
  currency: string;
  change: number;
  changePercent: number;
  updatedAt: string;
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
  /** "ready" | "cash" | "duplicate" | "invalid_symbol" | "validation_failed" */
  status: string;
}

export interface PreviewImportResult {
  rows: PreviewRow[];
  readyCount: number;
  skipCount: number;
}
// ── Tauri Command Signatures ──

// invoke('get_portfolio')           → PortfolioSnapshot
// invoke('get_holdings')            → Holding[]
// invoke('add_holding', { holding }) → Holding        (omit id, createdAt, updatedAt)
// invoke('update_holding', { holding }) → Holding
// invoke('delete_holding', { id })  → boolean
// invoke('refresh_prices')          → PriceData[]
// invoke('get_performance', { range }) → { date: string; value: number }[]
// invoke('run_stress_test_cmd', { scenario }) → StressResult
// invoke('search_symbols', { query }) → SymbolResult[]
// invoke('get_symbol_price', { symbol }) → PriceData
