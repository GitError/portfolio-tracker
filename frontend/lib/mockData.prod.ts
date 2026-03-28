/**
 * Production stub for mockData.ts.
 * In Tauri production builds, isTauri() is always true so mock data is never accessed.
 * This stub replaces the full mock module via Vite alias, keeping mock data out of the bundle.
 *
 * MOCK_SNAPSHOT must be a valid object (not null) because non-Tauri code paths spread it:
 * `{ ...MOCK_SNAPSHOT, lastUpdated: ... }`. A null value would throw at runtime.
 */
import type { Dividend, Holding, PortfolioSnapshot } from '../types/portfolio';

export const MOCK_SNAPSHOT: PortfolioSnapshot = {
  holdings: [],
  totalValue: 0,
  totalCost: 0,
  totalGainLoss: 0,
  totalGainLossPercent: 0,
  dailyPnl: 0,
  lastUpdated: new Date(0).toISOString(),
  baseCurrency: 'CAD',
  totalTargetWeight: 0,
  targetCashDelta: 0,
  realizedGains: 0,
  annualDividendIncome: 0,
};
export const MOCK_DIVIDENDS: Dividend[] = [];
export const MOCK_HOLDINGS: Holding[] = [];
