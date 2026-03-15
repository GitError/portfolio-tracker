import type { AccountType, StressScenario } from '../types/portfolio';

export interface PresetScenarioConfig extends StressScenario {
  description: string;
}

export function fxShockKey(currency: string, baseCurrency: string): string {
  return `fx_${currency.toLowerCase()}_${baseCurrency.toLowerCase()}`;
}

export function createPresetScenarios(baseCurrency: string): PresetScenarioConfig[] {
  const addFxShock = (shocks: Record<string, number>, currency: string, value: number) => {
    if (currency.toUpperCase() !== baseCurrency.toUpperCase()) {
      shocks[fxShockKey(currency, baseCurrency)] = value;
    }
  };

  const bearMarket: Record<string, number> = { stock: -0.2, etf: -0.2, crypto: -0.4 };
  addFxShock(bearMarket, 'USD', -0.05);

  const baseCurrencyDrop: Record<string, number> = {};
  addFxShock(baseCurrencyDrop, 'USD', 0.15);
  addFxShock(baseCurrencyDrop, 'EUR', 0.1);
  addFxShock(baseCurrencyDrop, 'GBP', 0.1);

  const stagflation: Record<string, number> = { stock: -0.15, etf: -0.12, crypto: -0.2 };
  addFxShock(stagflation, 'USD', 0.08);

  const aiCorrection: Record<string, number> = { stock: -0.15, etf: -0.12, crypto: -0.3 };
  addFxShock(aiCorrection, 'USD', -0.03);

  const techDrawdown: Record<string, number> = { stock: -0.25, etf: -0.18, crypto: -0.55 };
  addFxShock(techDrawdown, 'USD', 0.07);

  const inflationShock: Record<string, number> = { stock: -0.1, etf: -0.08, crypto: -0.15 };
  addFxShock(inflationShock, 'USD', 0.05);

  const cadWeakness: Record<string, number> = {};
  addFxShock(cadWeakness, 'USD', 0.12);
  addFxShock(cadWeakness, 'EUR', 0.08);
  addFxShock(cadWeakness, 'GBP', 0.08);

  const commodityRally: Record<string, number> = { stock: 0.03, etf: 0.02 };
  addFxShock(commodityRally, 'USD', -0.04);

  return [
    {
      name: 'Mild Correction',
      shocks: { stock: -0.05, etf: -0.05, crypto: -0.1 },
      description: 'Normal market pullback; risk assets slip 5–10% on profit-taking',
    },
    {
      name: 'Bear Market',
      shocks: bearMarket,
      description: 'Broad equity sell-off with crypto collapse; classic risk-off environment',
    },
    {
      name: 'Crypto Winter',
      shocks: { crypto: -0.5 },
      description: 'Crypto-specific collapse of 50%; equities and FX largely unaffected',
    },
    {
      name: 'Base Currency Drop',
      shocks: baseCurrencyDrop,
      description:
        'Domestic currency weakens sharply; foreign-denominated assets gain in local terms',
    },
    {
      name: 'Stagflation',
      shocks: stagflation,
      description:
        'High inflation + slowing growth; equities re-rate lower, USD firms on carry demand',
    },
    {
      name: 'AI Correction',
      shocks: aiCorrection,
      description:
        'Reversal of AI-driven valuations; growth and tech names hit hardest while USD softens',
    },
    {
      name: '2022 Tech Drawdown',
      shocks: techDrawdown,
      description: 'Rate shock rout — tech −40%, crypto −60%, USD strengthens on Fed hawkishness',
    },
    {
      name: 'Mild Recession',
      shocks: { stock: -0.12, etf: -0.1, crypto: -0.2 },
      description: 'Growth softens, earnings contract 10–15%; central banks slow to cut rates',
    },
    {
      name: 'Inflation Shock',
      shocks: inflationShock,
      description: 'Sticky inflation forces rate hikes; equities re-rate lower, USD firms on carry',
    },
    {
      name: 'CAD Weakness',
      shocks: cadWeakness,
      description:
        'Oil price slump + BoC dovishness weakens CAD; foreign holdings gain in CAD terms',
    },
    {
      name: 'Commodity Rally',
      shocks: commodityRally,
      description: 'Supply shock lifts energy/materials; CAD firms as petrocurrency',
    },
  ];
}

export const ASSET_TYPE_CONFIG = {
  stock: { label: 'Stock', color: 'var(--color-stock)', icon: 'TrendingUp' },
  etf: { label: 'ETF', color: 'var(--color-etf)', icon: 'BarChart2' },
  crypto: { label: 'Crypto', color: 'var(--color-crypto)', icon: 'Zap' },
  cash: { label: 'Cash', color: 'var(--color-cash)', icon: 'DollarSign' },
} as const;

export const ACCOUNT_OPTIONS: { value: AccountType; label: string }[] = [
  { value: 'tfsa', label: 'TFSA' },
  { value: 'rrsp', label: 'RRSP' },
  { value: 'taxable', label: 'Taxable' },
  { value: 'cash', label: 'Cash' },
];

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
