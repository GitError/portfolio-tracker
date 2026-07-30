import { describe, it, expect } from 'vitest';
import { summarizeDividendsBySymbol } from '../dividendSummary';
import type { Dividend, Holding } from '../../types/portfolio';

function makeHolding(
  overrides: Partial<Holding> & Pick<Holding, 'id' | 'symbol' | 'quantity'>
): Holding {
  return {
    name: overrides.symbol,
    assetType: 'stock',
    account: 'taxable',
    costBasis: 0,
    currency: 'CAD',
    exchange: 'TSX',
    targetWeight: 0,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    indicatedAnnualDividend: null,
    indicatedAnnualDividendCurrency: null,
    dividendFrequency: null,
    maturityDate: null,
    ...overrides,
  };
}

function makeDividend(
  overrides: Partial<Dividend> & Pick<Dividend, 'holdingId' | 'symbol'>
): Dividend {
  return {
    id: `div-${overrides.holdingId}-${overrides.payDate ?? '2026-01-31'}`,
    amountPerUnit: 0.1,
    currency: 'CAD',
    exDate: '2026-01-15',
    payDate: '2026-01-31',
    createdAt: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

describe('summarizeDividendsBySymbol', () => {
  it('multiplies amountPerUnit by the holding quantity, grouped by symbol', () => {
    const holdings = [makeHolding({ id: 'h1', symbol: 'TD.TO', quantity: 150 })];
    const dividends = [
      makeDividend({ holdingId: 'h1', symbol: 'TD.TO', amountPerUnit: 0.118, currency: 'CAD' }),
    ];

    const summary = summarizeDividendsBySymbol(dividends, holdings);

    expect(summary['TD.TO']).toEqual({ total: 17.7, currency: 'CAD', count: 1 });
  });

  it('sums multiple payments for the same symbol and counts them', () => {
    const holdings = [makeHolding({ id: 'h1', symbol: 'AAPL', quantity: 50 })];
    const dividends = [
      makeDividend({ holdingId: 'h1', symbol: 'AAPL', amountPerUnit: 0.25, currency: 'USD' }),
      makeDividend({ holdingId: 'h1', symbol: 'AAPL', amountPerUnit: 0.25, currency: 'USD' }),
    ];

    const summary = summarizeDividendsBySymbol(dividends, holdings);

    expect(summary['AAPL']).toEqual({ total: 25, currency: 'USD', count: 2 });
  });

  it('treats a missing holding as zero quantity rather than throwing', () => {
    const dividends = [makeDividend({ holdingId: 'missing', symbol: 'GONE', amountPerUnit: 1 })];

    const summary = summarizeDividendsBySymbol(dividends, []);

    expect(summary['GONE']).toEqual({ total: 0, currency: 'CAD', count: 1 });
  });

  it('documents the known #602 limitation: uses current quantity, not quantity at pay date', () => {
    // The holding was bought at 100 shares, then doubled to 200 shares *after* the dividend's
    // pay date. summarizeDividendsBySymbol has no per-payment quantity to fall back on, so it
    // uses the holding's current quantity (200) rather than the 100 shares actually held when
    // the dividend was paid. This test pins that known, documented behavior — if a future change
    // starts using pay-date quantity, this test should be updated to reflect the fix, and this
    // comment (and the one on summarizeDividendsBySymbol) should be removed.
    const holdings = [makeHolding({ id: 'h1', symbol: 'XYZ', quantity: 200 })];
    const dividends = [
      makeDividend({
        holdingId: 'h1',
        symbol: 'XYZ',
        amountPerUnit: 1,
        payDate: '2026-01-31',
        currency: 'CAD',
      }),
    ];

    const summary = summarizeDividendsBySymbol(dividends, holdings);

    // Actual cash received at pay date would have been 1 x 100 = 100, but the current
    // implementation reports 1 x 200 = 200 because it uses the current quantity.
    expect(summary['XYZ']).toEqual({ total: 200, currency: 'CAD', count: 1 });
  });
});
