/**
 * Production stub for perfMockData.ts.
 * In Tauri production builds, isTauri() is always true so perf mock data is never accessed.
 * This stub replaces the full mock module via Vite alias, keeping mock data out of the bundle.
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

export function generatePerfData(): PerfDataPoint[] {
  return [];
}

export const ALL_PERF_DATA: PerfDataPoint[] = [];
export const BENCHMARK_SERIES: BenchmarkSeries[] = [];

export interface PerfStats {
  totalReturn: number;
  cagr: number;
  maxDrawdown: number;
  volatility: number;
  sharpe: number;
  best: PerfDataPoint | null;
  worst: PerfDataPoint | null;
}

export function filterByRange(data: PerfDataPoint[], _range: string): PerfDataPoint[] {
  return data;
}

export function calcStats(_data: PerfDataPoint[]): PerfStats | null {
  return null;
}
