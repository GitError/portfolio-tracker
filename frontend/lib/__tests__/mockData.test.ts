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
});
