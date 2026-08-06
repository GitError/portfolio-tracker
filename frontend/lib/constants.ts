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

  // Historical event replays: shock values are derived from actual peak-to-trough
  // moves during the named event, not hypothetical estimates. Crypto is omitted
  // for pre-2009 events since it didn't exist yet.
  const globalFinancialCrisis: Record<string, number> = { stock: -0.567, etf: -0.567 };
  addFxShock(globalFinancialCrisis, 'USD', 0.21);

  const covidCrash: Record<string, number> = { stock: -0.339, etf: -0.339, crypto: -0.62 };
  addFxShock(covidCrash, 'USD', 0.11);

  const rateHike2022: Record<string, number> = { stock: -0.254, etf: -0.24, crypto: -0.78 };
  addFxShock(rateHike2022, 'USD', 0.1);

  const dotcomCrash: Record<string, number> = { stock: -0.49, etf: -0.45 };
  addFxShock(dotcomCrash, 'USD', 0.1);

  const blackMonday1987: Record<string, number> = { stock: -0.335, etf: -0.335 };

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
    {
      name: '2008 Global Financial Crisis',
      shocks: globalFinancialCrisis,
      description:
        'Replays the 2008 credit crisis using the actual peak-to-trough decline in broad equity markets and CAD depreciation against the US dollar. Crypto is excluded since it did not yet exist.',
      historicalParallel: 'Global Financial Crisis, Oct 2007 - Mar 2009',
      isHistorical: true,
      dataSource:
        'S&P 500 -56.8% peak-to-trough, Oct 9 2007 - Mar 9 2009; USD/CAD +21% over the same window',
    },
    {
      name: 'COVID-19 Crash',
      shocks: covidCrash,
      description:
        'Replays the fastest bear market on record: a five-week equity collapse alongside a sharp crypto selloff and a weaker CAD as investors rushed into US dollars.',
      historicalParallel: 'COVID-19 crash, Feb-Mar 2020',
      isHistorical: true,
      dataSource:
        'S&P 500 -33.9% peak-to-trough, Feb 19 2020 - Mar 23 2020; Bitcoin -62% over the same window (incl. Mar 12 2020 "Black Thursday"); USD/CAD +11%',
    },
    {
      name: '2022 Rate-Hike Cycle',
      shocks: rateHike2022,
      description:
        'Replays the 2022 drawdown driven by aggressive Fed tightening: equities fell over the full year while crypto suffered a much deeper collapse amid the Terra/Luna and FTX failures.',
      historicalParallel: '2022 rate hiking cycle',
      isHistorical: true,
      dataSource:
        'S&P 500 -25.4% peak-to-trough, Jan 3 2022 - Oct 12 2022; Bitcoin -77.6% peak-to-trough, Nov 2021 - Nov 2022; USD/CAD +10%',
    },
    {
      name: 'Dot-Com Crash',
      shocks: dotcomCrash,
      description:
        'Replays the 2000-2002 bear market that followed the internet stock bubble, with a multi-year equity decline and a weaker CAD. Crypto is excluded since it did not yet exist.',
      historicalParallel: 'Dot-com crash, Mar 2000 - Oct 2002',
      isHistorical: true,
      dataSource: 'S&P 500 -49% peak-to-trough, Mar 24 2000 - Oct 9 2002; USD/CAD +10%',
    },
    {
      name: 'Black Monday 1987',
      shocks: blackMonday1987,
      description:
        'Replays the 1987 crash centered on a single catastrophic trading day. FX and crypto shocks are excluded: reliable same-week CAD data is not available and crypto did not yet exist.',
      historicalParallel: 'Black Monday, Oct 19 1987',
      isHistorical: true,
      dataSource:
        'S&P 500 -33.5% peak-to-trough, Aug 25 1987 - Dec 4 1987 (incl. -20.5% on Oct 19 1987 alone)',
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
  { value: 'fhsa', label: 'FHSA' },
  { value: 'taxable', label: 'Taxable' },
  { value: 'crypto', label: 'Crypto' },
  { value: 'cash', label: 'Cash' },
  { value: 'other', label: 'Other' },
];

export const ACCOUNT_TYPE_CONFIG: Record<string, { label: string; color: string }> = {
  tfsa: { label: 'TFSA', color: 'var(--color-gain)' },
  rrsp: { label: 'RRSP', color: 'var(--color-accent)' },
  fhsa: { label: 'FHSA', color: '#8b5cf6' },
  taxable: { label: 'Taxable', color: '#f97316' },
  crypto: { label: 'Crypto', color: 'var(--color-crypto)' },
  cash: { label: 'Cash', color: 'var(--color-cash)' },
  other: { label: 'Other', color: 'var(--text-muted)' },
};

export const SUPPORTED_CURRENCIES = [
  'CAD',
  'USD',
  'EUR',
  'GBP',
  'CHF',
  'JPY',
  'AUD',
  'PLN',
] as const;

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
  AUD: '#10b981',
  PLN: '#e53e3e',
};
