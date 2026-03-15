import type { AccountType, StressScenario, StressScenarioInfo } from '../types/portfolio';

export interface PresetScenarioConfig extends StressScenario {
  description: string;
}

export function fxShockKey(currency: string, baseCurrency: string): string {
  return `fx_${currency.toLowerCase()}_${baseCurrency.toLowerCase()}`;
}

export function createPresetScenarioInfo(baseCurrency: string): StressScenarioInfo[] {
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
      description:
        'Models a routine pullback where equities fall modestly and crypto drops harder because of higher volatility.',
      historicalParallel: 'Q4 2018, Sep 2020',
    },
    {
      name: 'Bear Market',
      shocks: bearMarket,
      description:
        'Models a prolonged risk-off drawdown with large equity losses and a flight toward safety assets.',
      historicalParallel: '2008 GFC, 2022 rate hiking cycle',
    },
    {
      name: 'Crypto Winter',
      shocks: { crypto: -0.5 },
      description:
        'Models a crypto-specific collapse where digital assets reprice sharply while traditional assets stay relatively stable.',
      historicalParallel: '2018 crypto winter, May 2022 Terra/Luna collapse',
    },
    {
      name: 'Base Currency Drop',
      shocks: baseCurrencyDrop,
      description: `Models a sharp drop in ${baseCurrency} versus major currencies, increasing the local value of foreign holdings.`,
      historicalParallel: '2015 oil shock and CAD weakness',
    },
    {
      name: 'Stagflation',
      shocks: stagflation,
      description:
        'Models inflation staying high while growth weakens, pulling down risk assets while the local currency also softens.',
      historicalParallel: '1970s stagflation, partial parallel in 2022',
    },
    {
      name: 'AI Correction',
      shocks: aiCorrection,
      description:
        'Models a reversal in crowded AI and growth trades, with tech-heavy risk assets falling faster than the broader market.',
      historicalParallel: '2024 AI momentum unwind analogue',
    },
    {
      name: '2022 Tech Drawdown',
      shocks: techDrawdown,
      description:
        'Models a rate-shock-led technology selloff with deep crypto losses and a stronger USD.',
      historicalParallel: '2022 Nasdaq drawdown',
    },
    {
      name: 'Mild Recession',
      shocks: { stock: -0.12, etf: -0.1, crypto: -0.2 },
      description:
        'Models a moderate earnings recession where risk assets fall, but not to full bear-market extremes.',
      historicalParallel: '2001 shallow recession, 1990 soft landing miss',
    },
    {
      name: 'Inflation Shock',
      shocks: inflationShock,
      description:
        'Models sticky inflation forcing higher rates, weighing on equities while the USD strengthens.',
      historicalParallel: '2022 inflation repricing',
    },
    {
      name: 'CAD Weakness',
      shocks: cadWeakness,
      description:
        'Models a Canada-specific currency selloff that boosts the local-currency value of foreign assets.',
      historicalParallel: '2015-2016 CAD weakness',
    },
    {
      name: 'Commodity Rally',
      shocks: commodityRally,
      description:
        'Models a commodity-led upswing that helps resource-heavy equities while a stronger CAD offsets some foreign gains.',
      historicalParallel: '2021 energy and materials rally',
    },
  ];
}

export function createPresetScenarios(baseCurrency: string): StressScenario[] {
  return createPresetScenarioInfo(baseCurrency).map(({ name, shocks }) => ({ name, shocks }));
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

export const SUPPORTED_CURRENCIES = ['CAD', 'USD', 'EUR', 'GBP', 'CHF', 'JPY', 'AUD'] as const;

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
