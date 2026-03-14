import type { StressScenario } from '../types/portfolio';

export const PRESET_SCENARIOS: StressScenario[] = [
  {
    name: 'Mild Correction',
    shocks: { stock: -0.05, etf: -0.05, crypto: -0.1 },
  },
  {
    name: 'Bear Market',
    shocks: { stock: -0.2, etf: -0.2, crypto: -0.4, fx_usd_cad: -0.05 },
  },
  {
    name: 'Crypto Winter',
    shocks: { crypto: -0.5 },
  },
  {
    name: 'CAD Crash',
    shocks: { fx_usd_cad: 0.15, fx_eur_cad: 0.1, fx_gbp_cad: 0.1 },
  },
  {
    name: 'Stagflation',
    shocks: { stock: -0.15, etf: -0.12, crypto: -0.2, fx_usd_cad: 0.08 },
  },
];

export const ASSET_TYPE_CONFIG = {
  stock: { label: 'Stock', color: 'var(--color-stock)', icon: 'TrendingUp' },
  etf: { label: 'ETF', color: 'var(--color-etf)', icon: 'BarChart2' },
  crypto: { label: 'Crypto', color: 'var(--color-crypto)', icon: 'Zap' },
  cash: { label: 'Cash', color: 'var(--color-cash)', icon: 'DollarSign' },
} as const;

export const SUPPORTED_CURRENCIES = ['CAD', 'USD', 'EUR', 'GBP', 'CHF', 'JPY'] as const;

export const CHART_RANGES = [
  { label: '1W', value: '1W' },
  { label: '1M', value: '1M' },
  { label: '3M', value: '3M' },
  { label: '6M', value: '6M' },
  { label: '1Y', value: '1Y' },
] as const;

export const CURRENCY_COLORS: Record<string, string> = {
  CAD: '#00d4aa',
  USD: '#3b82f6',
  EUR: '#8b5cf6',
  GBP: '#f59e0b',
  CHF: '#f43f5e',
  JPY: '#ec4899',
};
