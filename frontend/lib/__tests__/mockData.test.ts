import { describe, it, expect } from 'vitest';
import { MOCK_SNAPSHOT, buildMockSnapshot, MOCK_HOLDINGS } from '../mockData';
import type { PortfolioSnapshot } from '../../types/portfolio';

describe('mockData', () => {
  it('MOCK_SNAPSHOT satisfies full PortfolioSnapshot interface', () => {
    const snapshot: PortfolioSnapshot = MOCK_SNAPSHOT;
    expect(snapshot).toBeDefined();
    expect(Array.isArray(snapshot.holdings)).toBe(true);
    expect(typeof snapshot.totalValue).toBe('number');
    expect(typeof snapshot.baseCurrency).toBe('string');
  });

  it('MOCK_SNAPSHOT has all required PortfolioSnapshot fields', () => {
    expect(typeof MOCK_SNAPSHOT.totalCost).toBe('number');
    expect(typeof MOCK_SNAPSHOT.totalGainLoss).toBe('number');
    expect(typeof MOCK_SNAPSHOT.totalGainLossPercent).toBe('number');
    expect(typeof MOCK_SNAPSHOT.dailyPnl).toBe('number');
    expect(typeof MOCK_SNAPSHOT.lastUpdated).toBe('string');
    expect(typeof MOCK_SNAPSHOT.totalTargetWeight).toBe('number');
    expect(typeof MOCK_SNAPSHOT.targetCashDelta).toBe('number');
    expect(typeof MOCK_SNAPSHOT.realizedGains).toBe('number');
    expect(typeof MOCK_SNAPSHOT.annualDividendIncome).toBe('number');
  });

  it('buildMockSnapshot returns a valid PortfolioSnapshot from a holding list', () => {
    const snapshot: PortfolioSnapshot = buildMockSnapshot(MOCK_HOLDINGS);
    expect(snapshot).toBeDefined();
    expect(Array.isArray(snapshot.holdings)).toBe(true);
    expect(snapshot.holdings).toHaveLength(MOCK_HOLDINGS.length);
    expect(typeof snapshot.totalValue).toBe('number');
    expect(typeof snapshot.baseCurrency).toBe('string');
  });

  it('buildMockSnapshot returns empty holdings array when given empty list', () => {
    const snapshot = buildMockSnapshot([]);
    expect(snapshot.holdings).toHaveLength(0);
    expect(snapshot.totalValue).toBe(0);
  });

  // Values below are hand-computed from the RAW_HOLDINGS fixture inputs in
  // mockData.ts (USD_CAD = 1.36, EUR_CAD = 1.47), independent of the
  // production formulas in buildSnapshot(), so a regression in either the
  // fixture data or the aggregation math will fail these.
  it('computes totalValue as the sum of every holding market value in CAD', () => {
    // 50*189.84*1.36 + 30*415.52*1.36 + 25*875.4*1.36 + 150*81.24 + 80*135.88
    //   + 40*481.55*1.36 + 200*112.4 + 0.85*87450 + 5*4380 + 8500*1.36
    //   + 3000*1.47 + 12500
    expect(MOCK_SNAPSHOT.totalValue).toBeCloseTo(256061.156, 2);
  });

  it("computes AAPL's gainLoss in CAD from its quantity, cost basis, and current price", () => {
    const aapl = MOCK_SNAPSHOT.holdings.find((h) => h.id === '1');
    // 50 * (189.84 - 155.0) * 1.36
    expect(aapl?.gainLoss).toBeCloseTo(2369.12, 2);
  });

  it("computes TD.TO's portfolio weight as its share of totalValue", () => {
    const td = MOCK_SNAPSHOT.holdings.find((h) => h.id === '4');
    // (150 * 81.24) / 256061.156 * 100
    expect(td?.weight).toBeCloseTo(4.759019, 4);
  });

  it('computes totalGainLoss as totalValue minus totalCost', () => {
    expect(MOCK_SNAPSHOT.totalGainLoss).toBeCloseTo(82801.156, 2);
    expect(MOCK_SNAPSHOT.totalGainLoss).toBeCloseTo(
      MOCK_SNAPSHOT.totalValue - MOCK_SNAPSHOT.totalCost,
      6
    );
  });
});
