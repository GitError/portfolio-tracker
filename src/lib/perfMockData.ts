export interface PerfDataPoint {
  date: string;       // YYYY-MM-DD
  value: number;      // CAD portfolio value
  dailyReturn: number; // % change vs previous day
}

function seededRng(seed: number) {
  let s = seed;
  return () => {
    s = (s * 1664525 + 1013904223) & 0xffffffff;
    return (s >>> 0) / 0xffffffff;
  };
}

export function generatePerfData(): PerfDataPoint[] {
  const rng = seededRng(42);
  const points: PerfDataPoint[] = [];
  let value = 180_000;

  const now = new Date();
  // 2 years of daily data
  const totalDays = 730;
  const start = new Date(now);
  start.setDate(start.getDate() - totalDays);

  // Define a crash window: days 200-220 = -15% drawdown
  const CRASH_START = 380;
  const CRASH_END = 400;

  for (let i = 0; i <= totalDays; i++) {
    const d = new Date(start);
    d.setDate(d.getDate() + i);

    // Skip weekends
    const dow = d.getDay();
    if (dow === 0 || dow === 6) continue;

    // Daily return: base trend + noise + crash
    let dailyPct = 0.03 + (rng() - 0.45) * 1.2; // ~+0.03% drift, ±0.6% noise

    // Occasional big moves
    if (rng() < 0.03) dailyPct += (rng() - 0.5) * 4;

    // Crash period
    if (i >= CRASH_START && i < CRASH_END) dailyPct = -0.9 - rng() * 0.4;
    // Recovery after crash
    if (i >= CRASH_END && i < CRASH_END + 30) dailyPct = 0.4 + rng() * 0.3;

    value = value * (1 + dailyPct / 100);

    const dateStr = d.toISOString().split('T')[0];
    points.push({ date: dateStr, value: Math.round(value * 100) / 100, dailyReturn: dailyPct });
  }

  return points;
}

export const ALL_PERF_DATA: PerfDataPoint[] = generatePerfData();

export function filterByRange(data: PerfDataPoint[], range: string): PerfDataPoint[] {
  const now = new Date();
  const cutoff = new Date(now);

  switch (range) {
    case '1D': cutoff.setDate(cutoff.getDate() - 2);   break;
    case '1W': cutoff.setDate(cutoff.getDate() - 7);   break;
    case '1M': cutoff.setMonth(cutoff.getMonth() - 1); break;
    case '3M': cutoff.setMonth(cutoff.getMonth() - 3); break;
    case '6M': cutoff.setMonth(cutoff.getMonth() - 6); break;
    case '1Y': cutoff.setFullYear(cutoff.getFullYear() - 1); break;
    case 'ALL': return data;
    default: cutoff.setMonth(cutoff.getMonth() - 1);
  }

  return data.filter((p) => new Date(p.date) >= cutoff);
}

export interface PerfStats {
  totalReturn: number;
  totalReturnPct: number;
  periodHigh: number;
  periodLow: number;
  maxDrawdown: number;
  volatility: number;
  bestDay: { date: string; pct: number };
  worstDay: { date: string; pct: number };
}

export function calcStats(data: PerfDataPoint[]): PerfStats | null {
  if (data.length < 2) return null;

  const first = data[0].value;
  const last = data[data.length - 1].value;
  const totalReturn = last - first;
  const totalReturnPct = (totalReturn / first) * 100;

  const periodHigh = Math.max(...data.map((d) => d.value));
  const periodLow = Math.min(...data.map((d) => d.value));

  // Max drawdown: largest peak-to-trough
  let maxDrawdown = 0;
  let peak = data[0].value;
  for (const p of data) {
    if (p.value > peak) peak = p.value;
    const dd = (peak - p.value) / peak;
    if (dd > maxDrawdown) maxDrawdown = dd;
  }

  // Annualized volatility (std dev of daily returns × sqrt(252))
  const returns = data.slice(1).map((p) => p.dailyReturn);
  const mean = returns.reduce((s, r) => s + r, 0) / returns.length;
  const variance = returns.reduce((s, r) => s + (r - mean) ** 2, 0) / returns.length;
  const volatility = Math.sqrt(variance) * Math.sqrt(252);

  const bestDay = data.reduce((a, b) => (b.dailyReturn > a.dailyReturn ? b : a));
  const worstDay = data.reduce((a, b) => (b.dailyReturn < a.dailyReturn ? b : a));

  return {
    totalReturn,
    totalReturnPct,
    periodHigh,
    periodLow,
    maxDrawdown: maxDrawdown * 100,
    volatility,
    bestDay: { date: bestDay.date, pct: bestDay.dailyReturn },
    worstDay: { date: worstDay.date, pct: worstDay.dailyReturn },
  };
}
