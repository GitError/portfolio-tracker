import type { AccountType, StressScenario, StressScenarioInfo } from '../types/portfolio';

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
