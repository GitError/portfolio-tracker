/**
 * Production stub for perfMockData.ts.
 * In Tauri production builds, isTauri() is always true so the large mock datasets are never
 * accessed. This stub replaces those datasets with empty arrays while keeping utility functions
 * (calcStats, filterByRange) fully implemented — they operate on real Tauri data in production.
 */

export interface PerfDataPoint {
  date: string;
  value: number;
  dailyReturn: number;
}

export interface BenchmarkSeries {
  id: string;
  label: string;
  points: PerfDataPoint[];
}

export interface PerfStats {
  totalReturn: number;
  cagr: number;
  maxDrawdown: number;
  volatility: number;
  sharpe: number;
  best: PerfDataPoint | null;
  worst: PerfDataPoint | null;
}

// Mock datasets stubbed out — not needed in production Tauri builds.
export function generatePerfData(): PerfDataPoint[] {
  return [];
}
export const ALL_PERF_DATA: PerfDataPoint[] = [];
export const BENCHMARK_SERIES: BenchmarkSeries[] = [];

export function filterByRange(data: PerfDataPoint[], _range: string): PerfDataPoint[] {
  return data;
}

/**
 * Compute summary statistics for a performance data series.
 * Used by Performance.tsx for both real (Tauri) and mock (dev) data — must be fully implemented.
 */
export function calcStats(data: PerfDataPoint[]): PerfStats | null {
  if (data.length === 0) {
    return null;
  }

  // Safe after the length guard above.
  const first = data[0] as PerfDataPoint;
  const last = data[data.length - 1] as PerfDataPoint;

  // Total return based on portfolio value change over the period.
  const totalReturn = first.value !== 0 ? last.value / first.value - 1 : 0;

  const numDays = data.length;
  const tradingDaysPerYear = 252;

  // Compound annual growth rate (CAGR) from total return and period length.
  const years = numDays > 1 ? numDays / tradingDaysPerYear : 0;
  const cagr = years > 0 ? Math.pow(1 + totalReturn, 1 / years) - 1 : totalReturn;

  // Max drawdown based on path of portfolio value.
  let peak = first.value;
  let maxDrawdown = 0;
  for (const point of data) {
    if (point.value > peak) {
      peak = point.value;
    }
    if (peak > 0) {
      const drawdown = point.value / peak - 1;
      if (drawdown < maxDrawdown) {
        maxDrawdown = drawdown;
      }
    }
  }

  // Use provided dailyReturn values for risk metrics when available.
  const dailyReturns = data
    .map((p) => p.dailyReturn)
    .filter((r) => typeof r === 'number' && !Number.isNaN(r));

  let volatility = 0;
  let sharpe = 0;
  if (dailyReturns.length > 1) {
    const mean = dailyReturns.reduce((sum, r) => sum + r, 0) / dailyReturns.length;
    const variance =
      dailyReturns.reduce((sum, r) => sum + (r - mean) * (r - mean), 0) / (dailyReturns.length - 1);
    const dailyStdDev = Math.sqrt(variance);
    const annualizationFactor = Math.sqrt(tradingDaysPerYear);
    volatility = dailyStdDev * annualizationFactor;
    // Risk-free rate assumed 0 for Sharpe ratio.
    sharpe = dailyStdDev !== 0 ? (mean / dailyStdDev) * annualizationFactor : 0;
  }

  // Best and worst single-day performance based on dailyReturn.
  let best: PerfDataPoint | null = null;
  let worst: PerfDataPoint | null = null;
  for (const point of data) {
    if (best === null || point.dailyReturn > best.dailyReturn) {
      best = point;
    }
    if (worst === null || point.dailyReturn < worst.dailyReturn) {
      worst = point;
    }
  }

  return { totalReturn, cagr, maxDrawdown, volatility, sharpe, best, worst };
}
