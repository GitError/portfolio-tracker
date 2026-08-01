import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import { ResilienceSummary } from '../ResilienceSummary';
import type { HoldingWithPrice, PortfolioSnapshot } from '../../types/portfolio';

// Initialize i18n (ResilienceSummary uses useFormatNumber, which subscribes to i18next)
import i18next from '../../lib/i18n';

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
    targetWeight: null,
    weight: 12.34,
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
    fxStale: false,
    priceIsStale: false,
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
    totalTargetWeight: 0,
    targetCashDelta: 0,
    realizedGains: 0,
    annualDividendIncome: 0,
  };
}

afterEach(async () => {
  await act(async () => {
    await i18next.changeLanguage('en');
  });
});

describe('ResilienceSummary locale-aware formatting', () => {
  it('renders the largest position weight with German locale separators', async () => {
    await act(async () => {
      await i18next.changeLanguage('de');
    });

    const holding = makeHolding({ weight: 12.34 });
    render(<ResilienceSummary portfolio={makeSnapshot([holding])} />);

    // weight * 100 = 1234 -> "1.234,0" in de-DE (period thousands, comma decimal)
    expect(screen.getByText('1.234,0%')).toBeTruthy();
    // Guard against the pre-fix behavior (raw toFixed() always uses '.' regardless of locale).
    expect(screen.queryByText('1234.0%')).toBeNull();
  });
});
