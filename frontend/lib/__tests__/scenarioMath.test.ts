import { describe, it, expect } from 'vitest';
import { computeStressImpact } from '../scenarioMath';
import type {
  AssetType,
  HoldingWithPrice,
  PortfolioSnapshot,
  StressScenario,
} from '../../types/portfolio';

function makeHolding(
  symbol: string,
  assetType: AssetType,
  currency: string,
  value: number
): HoldingWithPrice {
  return {
    id: symbol,
    symbol,
    name: symbol,
    assetType,
    account: assetType === 'cash' ? 'cash' : 'taxable',
    quantity: 1,
    costBasis: value,
    currency,
    exchange: '',
    targetWeight: null,
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
    indicatedAnnualDividend: null,
    indicatedAnnualDividendCurrency: null,
    dividendFrequency: null,
    maturityDate: null,
    currentPrice: value,
    currentPriceCad: value,
    marketValueCad: value,
    costValueCad: value,
    gainLoss: 0,
    gainLossPercent: 0,
    weight: 1,
    targetValue: 0,
    targetDeltaValue: 0,
    targetDeltaPercent: 0,
    dailyChangePercent: 0,
    fxStale: false,
    priceIsStale: false,
  };
}

function makeSnapshot(holdings: HoldingWithPrice[], baseCurrency = 'CAD'): PortfolioSnapshot {
  const total = holdings.reduce((sum, h) => sum + h.marketValueCad, 0);
  return {
    holdings,
    totalValue: total,
    totalCost: total,
    totalGainLoss: 0,
    totalGainLossPercent: 0,
    dailyPnl: 0,
    lastUpdated: '2024-01-01T00:00:00Z',
    baseCurrency,
    totalTargetWeight: 0,
    targetCashDelta: 0,
    realizedGains: 0,
    annualDividendIncome: 0,
  };
}

describe('computeStressImpact', () => {
  it('returns unchanged values for zero shocks', () => {
    const snapshot = makeSnapshot([
      makeHolding('AAPL', 'stock', 'USD', 10_000),
      makeHolding('BTC', 'crypto', 'CAD', 5_000),
    ]);
    const scenario: StressScenario = { name: 'Zero', shocks: {} };

    const result = computeStressImpact(snapshot, scenario);

    expect(Math.abs(result.totalImpact)).toBeLessThan(0.001);
    expect(result.holdingBreakdown).toHaveLength(2);
    for (const h of result.holdingBreakdown) {
      expect(Math.abs(h.impact)).toBeLessThan(0.001);
    }
  });

  it('applies a stock shock correctly', () => {
    const value = 10_000;
    const snapshot = makeSnapshot([makeHolding('AAPL', 'stock', 'CAD', value)]);
    const scenario: StressScenario = { name: 'Bear', shocks: { stock: -0.2 } };

    const result = computeStressImpact(snapshot, scenario);

    expect(result.stressedValue).toBeCloseTo(value * 0.8, 3);
    expect(result.totalImpact).toBeCloseTo(-2_000, 3);
  });

  it('applies FX shock to non-base-currency holdings', () => {
    const value = 10_000;
    const snapshot = makeSnapshot([makeHolding('AAPL', 'stock', 'USD', value)]);
    const scenario: StressScenario = {
      name: 'Mixed',
      shocks: { stock: -0.1, fx_usd_cad: 0.05 },
    };

    const result = computeStressImpact(snapshot, scenario);

    expect(result.stressedValue).toBeCloseTo(value * 0.9 * 1.05, 3);
  });

  it('ignores FX shocks for base-currency holdings', () => {
    const value = 5_000;
    const snapshot = makeSnapshot([makeHolding('RY.TO', 'stock', 'CAD', value)]);
    const scenario: StressScenario = { name: 'FX only', shocks: { fx_usd_cad: 0.15 } };

    const result = computeStressImpact(snapshot, scenario);

    expect(Math.abs(result.totalImpact)).toBeLessThan(0.001);
  });

  it('excludes cash from asset shocks but still applies FX shocks to it', () => {
    const value = 10_000;
    const snapshot = makeSnapshot([
      makeHolding('USD-CASH', 'cash', 'USD', value),
      makeHolding('CAD-CASH', 'cash', 'CAD', value),
    ]);
    const scenario: StressScenario = {
      name: 'Cash test',
      shocks: { cash: -0.5, fx_usd_cad: 0.1 },
    };

    const result = computeStressImpact(snapshot, scenario);

    const usd = result.holdingBreakdown.find((h) => h.symbol === 'USD-CASH')!;
    const cad = result.holdingBreakdown.find((h) => h.symbol === 'CAD-CASH')!;

    expect(usd.stressedValue).toBeCloseTo(11_000, 3);
    expect(cad.stressedValue).toBeCloseTo(value, 3);
  });

  it('does not let stressed value go negative on a total-loss shock', () => {
    const value = 10_000;
    const snapshot = makeSnapshot([makeHolding('BTC', 'crypto', 'CAD', value)]);
    const scenario: StressScenario = { name: 'Crypto gone', shocks: { crypto: -1.0 } };

    const result = computeStressImpact(snapshot, scenario);

    expect(result.stressedValue).toBeGreaterThanOrEqual(0);
    expect(result.stressedValue).toBeCloseTo(0, 3);
    expect(result.totalImpact).toBeCloseTo(-value, 3);
  });

  it('handles an empty portfolio without producing NaN', () => {
    const snapshot = makeSnapshot([]);
    const scenario: StressScenario = { name: 'Empty', shocks: {} };

    const result = computeStressImpact(snapshot, scenario);

    expect(result.totalImpact).toBe(0);
    expect(result.totalImpactPercent).toBe(0);
  });
});
