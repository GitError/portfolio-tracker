import { describe, it, expect } from 'vitest';
import { buildInsights } from '../useActionInsights';
import type { HoldingWithPrice, PortfolioSnapshot } from '../../types/portfolio';

// ─── Test helpers ─────────────────────────────────────────────────────────────

function makeHolding(overrides: Partial<HoldingWithPrice> = {}): HoldingWithPrice {
  return {
    id: 'h1',
    symbol: 'AAPL',
    name: 'Apple',
    assetType: 'stock',
    account: 'taxable',
    quantity: 10,
    costBasis: 150,
    currency: 'USD',
    exchange: 'NASDAQ',
    targetWeight: 0,
    weight: 0.1,
    currentPrice: 200,
    currentPriceCad: 280,
    marketValueCad: 2800,
    costValueCad: 2100,
    gainLoss: 700,
    gainLossPercent: 33.3,
    targetValue: 0,
    targetDeltaValue: 0,
    targetDeltaPercent: 0,
    dailyChangePercent: 0.5,
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
    indicatedAnnualDividend: null,
    indicatedAnnualDividendCurrency: null,
    dividendFrequency: null,
    maturityDate: null,
    ...overrides,
  };
}

function makeSnapshot(holdings: HoldingWithPrice[]): PortfolioSnapshot {
  const totalValue = holdings.reduce((s, h) => s + h.marketValueCad, 0);
  return {
    holdings,
    totalValue,
    totalCost: 0,
    totalGainLoss: 0,
    totalGainLossPercent: 0,
    dailyPnl: 0,
    lastUpdated: '2024-01-01T00:00:00Z',
    baseCurrency: 'CAD',
    totalTargetWeight: 100,
    targetCashDelta: 0,
    realizedGains: 0,
    annualDividendIncome: 0,
  };
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe('buildInsights', () => {
  // ── 1. Empty holdings ──────────────────────────────────────────────────────
  it('returns empty array when holdings array is empty', () => {
    const snapshot = makeSnapshot([]);
    expect(buildInsights(snapshot, [])).toEqual([]);
  });

  // ── 2. Zero total value ────────────────────────────────────────────────────
  it('returns empty array when totalValue is 0', () => {
    const holding = makeHolding({ marketValueCad: 0 });
    const snapshot = { ...makeSnapshot([holding]), totalValue: 0 };
    expect(buildInsights(snapshot, [holding])).toEqual([]);
  });

  // ── 3. target_drift: drift < 5% → no insight ──────────────────────────────
  it('target_drift: drift < 5% produces no insight', () => {
    // targetWeight=10 (10%), weight=0.12 (12%) → drift=0.02 (2%)
    const holding = makeHolding({ targetWeight: 10, weight: 0.12, marketValueCad: 1200 });
    const snapshot = makeSnapshot([holding]);
    const insights = buildInsights(snapshot, [holding]);
    expect(insights.some((i) => i.type === 'target_drift')).toBe(false);
  });

  // ── 4. target_drift: drift 6% → warning ───────────────────────────────────
  it('target_drift: drift of 6% produces warning severity', () => {
    // targetWeight=10 (10%), weight=0.16 (16%) → drift=0.06 (6%) → warning
    const holding = makeHolding({
      id: 'h1',
      symbol: 'AAPL',
      targetWeight: 10,
      weight: 0.16,
      marketValueCad: 1600,
    });
    const snapshot = makeSnapshot([holding]);
    const insights = buildInsights(snapshot, [holding]);
    const drift = insights.find((i) => i.type === 'target_drift');
    expect(drift).toBeDefined();
    expect(drift!.severity).toBe('warning');
  });

  // ── 5. target_drift: drift 15% → critical ─────────────────────────────────
  it('target_drift: drift of 15% produces critical severity', () => {
    // targetWeight=10 (10%), weight=0.25 (25%) → drift=0.15 (15%) → critical
    const holding = makeHolding({
      id: 'h1',
      symbol: 'AAPL',
      targetWeight: 10,
      weight: 0.25,
      marketValueCad: 2500,
    });
    const snapshot = makeSnapshot([holding]);
    const insights = buildInsights(snapshot, [holding]);
    const drift = insights.find((i) => i.type === 'target_drift');
    expect(drift).toBeDefined();
    expect(drift!.severity).toBe('critical');
  });

  // ── 6. target_drift: direction 'overweight' ────────────────────────────────
  it('target_drift: direction is overweight when weight > targetFraction', () => {
    // targetWeight=10 → targetFraction=0.10; weight=0.20 → overweight
    const holding = makeHolding({
      id: 'h1',
      symbol: 'AAPL',
      targetWeight: 10,
      weight: 0.2,
      marketValueCad: 2000,
    });
    const snapshot = makeSnapshot([holding]);
    const insights = buildInsights(snapshot, [holding]);
    const drift = insights.find((i) => i.type === 'target_drift');
    expect(drift!.title).toContain('overweight');
  });

  // ── 7. target_drift: direction 'underweight' ───────────────────────────────
  it('target_drift: direction is underweight when weight < targetFraction', () => {
    // targetWeight=30 → targetFraction=0.30; weight=0.10 → underweight
    const holding = makeHolding({
      id: 'h1',
      symbol: 'AAPL',
      targetWeight: 30,
      weight: 0.1,
      marketValueCad: 1000,
    });
    const snapshot = makeSnapshot([holding]);
    const insights = buildInsights(snapshot, [holding]);
    const drift = insights.find((i) => i.type === 'target_drift');
    expect(drift!.title).toContain('underweight');
  });

  // ── 8. target_drift: correct display values ────────────────────────────────
  it('target_drift: metrics contain correct actualPct, targetPct, driftPct strings', () => {
    // targetWeight=20 (20%), weight=0.32 (32%) → drift=0.12 (12%)
    const holding = makeHolding({
      id: 'h1',
      symbol: 'AAPL',
      targetWeight: 20,
      weight: 0.32,
      marketValueCad: 3200,
    });
    const snapshot = makeSnapshot([holding]);
    const insights = buildInsights(snapshot, [holding]);
    const drift = insights.find((i) => i.type === 'target_drift');
    expect(drift!.metrics?.current).toBe('32.0'); // actualPct
    expect(drift!.metrics?.target).toBe('20.0'); // targetPct (targetWeight.toFixed(1))
    expect(drift!.metrics?.drift).toBe('12.0'); // driftPct
  });

  // ── 9. concentration_risk: weight < 30% → no insight ─────────────────────
  it('concentration_risk: weight < 30% produces no concentration insight', () => {
    // weight=0.25 → weightPct=25% ≤ 30
    const holding = makeHolding({ weight: 0.25, marketValueCad: 2500 });
    const snapshot = makeSnapshot([holding]);
    const insights = buildInsights(snapshot, [holding]);
    expect(insights.some((i) => i.type === 'concentration_risk')).toBe(false);
  });

  // ── 10. concentration_risk: weight exactly 30% → no insight ──────────────
  it('concentration_risk: weight exactly 30% produces no concentration insight', () => {
    const holding = makeHolding({ weight: 0.3, marketValueCad: 3000 });
    const snapshot = makeSnapshot([holding]);
    const insights = buildInsights(snapshot, [holding]);
    expect(insights.some((i) => i.type === 'concentration_risk')).toBe(false);
  });

  // ── 11. concentration_risk: weight 35% → warning ─────────────────────────
  it('concentration_risk: weight 35% produces warning severity', () => {
    const holding = makeHolding({ weight: 0.35, marketValueCad: 3500 });
    const snapshot = makeSnapshot([holding]);
    const insights = buildInsights(snapshot, [holding]);
    const conc = insights.find((i) => i.type === 'concentration_risk');
    expect(conc).toBeDefined();
    expect(conc!.severity).toBe('warning');
  });

  // ── 12. concentration_risk: weight 55% → critical ────────────────────────
  it('concentration_risk: weight 55% produces critical severity', () => {
    const holding = makeHolding({ weight: 0.55, marketValueCad: 5500 });
    const snapshot = makeSnapshot([holding]);
    const insights = buildInsights(snapshot, [holding]);
    const conc = insights.find((i) => i.type === 'concentration_risk');
    expect(conc).toBeDefined();
    expect(conc!.severity).toBe('critical');
  });

  // ── 13. idle_cash: cash allocation < 20% → no insight ────────────────────
  it('idle_cash: cash < 20% of portfolio produces no insight', () => {
    const stock = makeHolding({
      id: 'h1',
      symbol: 'AAPL',
      assetType: 'stock',
      weight: 0.85,
      marketValueCad: 8500,
    });
    const cash = makeHolding({
      id: 'h2',
      symbol: 'CASH-CAD',
      assetType: 'cash',
      weight: 0.15,
      marketValueCad: 1500,
    });
    const holdings = [stock, cash];
    const snapshot = makeSnapshot(holdings);
    const insights = buildInsights(snapshot, holdings);
    expect(insights.some((i) => i.type === 'idle_cash')).toBe(false);
  });

  // ── 14. idle_cash: cash allocation > 20% → info insight ─────────────────
  it('idle_cash: cash > 20% of portfolio produces info insight', () => {
    const stock = makeHolding({
      id: 'h1',
      symbol: 'AAPL',
      assetType: 'stock',
      weight: 0.7,
      marketValueCad: 7000,
    });
    const cash = makeHolding({
      id: 'h2',
      symbol: 'CASH-CAD',
      assetType: 'cash',
      weight: 0.3,
      marketValueCad: 3000,
    });
    const holdings = [stock, cash];
    const snapshot = makeSnapshot(holdings);
    const insights = buildInsights(snapshot, holdings);
    const idle = insights.find((i) => i.type === 'idle_cash');
    expect(idle).toBeDefined();
    expect(idle!.severity).toBe('info');
  });

  // ── 15. missing_targets: fewer than half missing → no insight ─────────────
  it('missing_targets: fewer than half missing produces no insight', () => {
    // 2 holdings with targets, 1 without → 1 of 3 missing (< 50%)
    const h1 = makeHolding({
      id: 'h1',
      symbol: 'AAPL',
      targetWeight: 30,
      weight: 0.33,
      marketValueCad: 3300,
    });
    const h2 = makeHolding({
      id: 'h2',
      symbol: 'MSFT',
      targetWeight: 30,
      weight: 0.33,
      marketValueCad: 3300,
    });
    const h3 = makeHolding({
      id: 'h3',
      symbol: 'GOOG',
      targetWeight: 0,
      weight: 0.34,
      marketValueCad: 3400,
    });
    const holdings = [h1, h2, h3];
    const snapshot = makeSnapshot(holdings);
    const insights = buildInsights(snapshot, holdings);
    expect(insights.some((i) => i.type === 'missing_targets')).toBe(false);
  });

  // ── 16. missing_targets: more than half missing → info insight ────────────
  it('missing_targets: more than half missing produces info insight', () => {
    // 1 holding with target, 2 without → 2 of 3 missing (> 50%)
    const h1 = makeHolding({
      id: 'h1',
      symbol: 'AAPL',
      targetWeight: 30,
      weight: 0.33,
      marketValueCad: 3300,
    });
    const h2 = makeHolding({
      id: 'h2',
      symbol: 'MSFT',
      targetWeight: 0,
      weight: 0.33,
      marketValueCad: 3300,
    });
    const h3 = makeHolding({
      id: 'h3',
      symbol: 'GOOG',
      targetWeight: 0,
      weight: 0.34,
      marketValueCad: 3400,
    });
    const holdings = [h1, h2, h3];
    const snapshot = makeSnapshot(holdings);
    const insights = buildInsights(snapshot, holdings);
    const missing = insights.find((i) => i.type === 'missing_targets');
    expect(missing).toBeDefined();
    expect(missing!.severity).toBe('info');
  });

  // ── 17. Sort order: critical before warning before info ───────────────────
  it('sorts insights critical → warning → info', () => {
    // h1: 55% weight → critical concentration_risk
    // h2: 35% weight → warning concentration_risk, but total is > 100% — use separate total
    // We need a scenario that naturally produces all three severities.
    // Use:
    //   - h1 with weight 0.55 → concentration_risk critical (weightPct > 50)
    //   - h2 with weight 0.35 → concentration_risk warning (weightPct 30–50)
    //   - cash > 20% of totalValue → idle_cash info
    // For a self-consistent snapshot we use a large total value so each
    // holding's marketValueCad / totalValue gives the desired weight.
    const h1 = makeHolding({
      id: 'h1',
      symbol: 'AAPL',
      assetType: 'stock',
      weight: 0.55,
      marketValueCad: 11000,
    });
    const h2 = makeHolding({
      id: 'h2',
      symbol: 'MSFT',
      assetType: 'stock',
      weight: 0.35,
      marketValueCad: 7000,
    });
    const cash = makeHolding({
      id: 'h3',
      symbol: 'CASH-CAD',
      assetType: 'cash',
      weight: 0.1,
      marketValueCad: 2000,
    });
    const holdings = [h1, h2, cash];
    // Override totalValue so idle_cash is triggered: cashPct = 2000 / 10000 = 20% (not > 20)
    // Use totalValue = 8000 so cashPct = 2000/8000 = 25%
    const snapshot: PortfolioSnapshot = {
      ...makeSnapshot(holdings),
      totalValue: 8000,
    };
    const insights = buildInsights(snapshot, holdings);
    const severities = insights.map((i) => i.severity);
    // Find first critical, first warning, first info positions
    const criticalIdx = severities.indexOf('critical');
    const warningIdx = severities.indexOf('warning');
    const infoIdx = severities.indexOf('info');
    expect(criticalIdx).toBeGreaterThanOrEqual(0);
    expect(warningIdx).toBeGreaterThanOrEqual(0);
    expect(infoIdx).toBeGreaterThanOrEqual(0);
    expect(criticalIdx).toBeLessThan(warningIdx);
    expect(warningIdx).toBeLessThan(infoIdx);
  });
});
