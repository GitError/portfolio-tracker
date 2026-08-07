import { describe, it, expect, beforeEach } from 'vitest';
import { loadCachedPortfolio, saveCachedPortfolio, clearSnapshotCache } from '../portfolioCache';
import type { PortfolioSnapshot } from '../../types/portfolio';

const CACHE_KEY = 'portfolio_snapshot_cache';

function buildSnapshot(overrides: Partial<PortfolioSnapshot> = {}): PortfolioSnapshot {
  return {
    holdings: [],
    totalValue: 10_000,
    totalCost: 9_000,
    totalGainLoss: 1_000,
    totalGainLossPercent: 11.1,
    dailyPnl: 50,
    lastUpdated: '2026-01-01T00:00:00Z',
    baseCurrency: 'CAD',
    totalTargetWeight: 100,
    targetCashDelta: 0,
    realizedGains: 0,
    annualDividendIncome: 0,
    ...overrides,
  };
}

beforeEach(() => {
  localStorage.clear();
});

describe('saveCachedPortfolio', () => {
  it('persists only totals — never the per-holding array', () => {
    const snapshot = buildSnapshot({
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      holdings: [{ id: '1', symbol: 'AAPL', costBasis: 12345 } as any],
    });

    saveCachedPortfolio(snapshot);

    const raw = localStorage.getItem(CACHE_KEY);
    expect(raw).not.toBeNull();
    const parsed = JSON.parse(raw!);
    expect(parsed).toEqual({
      totalValue: 10_000,
      holdingCount: 1,
      lastUpdated: '2026-01-01T00:00:00Z',
      baseCurrency: 'CAD',
    });
    // The sensitive per-holding fields (symbol, costBasis) must not appear anywhere in the cache.
    expect(raw).not.toContain('AAPL');
    expect(raw).not.toContain('12345');
  });
});

describe('loadCachedPortfolio', () => {
  it('round-trips a saved snapshot', () => {
    saveCachedPortfolio(buildSnapshot({ totalValue: 5_000, holdings: [] }));

    const cached = loadCachedPortfolio();

    expect(cached).toEqual({
      totalValue: 5_000,
      holdingCount: 0,
      lastUpdated: '2026-01-01T00:00:00Z',
      baseCurrency: 'CAD',
    });
  });

  it('returns null when nothing is cached', () => {
    expect(loadCachedPortfolio()).toBeNull();
  });

  it('returns null on corrupt JSON without throwing', () => {
    localStorage.setItem(CACHE_KEY, '{not valid json');

    expect(loadCachedPortfolio()).toBeNull();
  });

  it('returns null and clears storage when the schema does not match (e.g. legacy full-snapshot cache)', () => {
    // Legacy shape from before the localStorage footprint was reduced (#757).
    localStorage.setItem(
      CACHE_KEY,
      JSON.stringify({ snapshot: buildSnapshot(), holdings: [{ id: '1' }] })
    );

    expect(loadCachedPortfolio()).toBeNull();
    expect(localStorage.getItem(CACHE_KEY)).toBeNull();
  });
});

describe('clearSnapshotCache', () => {
  it('removes the cached entry', () => {
    saveCachedPortfolio(buildSnapshot());
    expect(loadCachedPortfolio()).not.toBeNull();

    clearSnapshotCache();

    expect(loadCachedPortfolio()).toBeNull();
  });

  it('is a no-op when nothing is cached', () => {
    expect(() => clearSnapshotCache()).not.toThrow();
  });
});
