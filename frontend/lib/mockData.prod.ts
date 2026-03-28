/**
 * Production stub for mockData.ts.
 * In Tauri production builds, isTauri() is always true so mock data is never accessed.
 * This stub replaces the full mock module via Vite alias, keeping mock data out of the bundle.
 */
import type { Dividend, Holding, PortfolioSnapshot } from '../types/portfolio';

export const MOCK_SNAPSHOT: PortfolioSnapshot = null as unknown as PortfolioSnapshot;
export const MOCK_DIVIDENDS: Dividend[] = [];
export const MOCK_HOLDINGS: Holding[] = [];
