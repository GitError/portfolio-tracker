import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { TopBar } from '../TopBar';
import type { PortfolioSnapshot } from '../../types/portfolio';

// Initialize i18n (TopBar uses useTranslation)
import i18next from '../../lib/i18n';

const mockPortfolio: PortfolioSnapshot = {
  holdings: [],
  totalValue: 1000,
  totalCost: 900,
  totalGainLoss: 100,
  totalGainLossPercent: 11.1,
  dailyPnl: 10,
  lastUpdated: '2024-03-05T14:30:00Z',
  baseCurrency: 'CAD',
  totalTargetWeight: 0,
  targetCashDelta: 0,
  realizedGains: 0,
  annualDividendIncome: 0,
};

afterEach(async () => {
  await act(async () => {
    await i18next.changeLanguage('en');
  });
});

function renderTopBar(overrides: Partial<React.ComponentProps<typeof TopBar>> = {}) {
  return render(
    <MemoryRouter>
      <TopBar
        portfolio={mockPortfolio}
        loading={false}
        isOffline={true}
        onRefresh={vi.fn()}
        baseCurrency="CAD"
        onBaseCurrencyChange={vi.fn()}
        {...overrides}
      />
    </MemoryRouter>
  );
}

describe('TopBar offline banner locale-aware date formatting', () => {
  it('formats the last-updated timestamp using the active i18next locale, not the environment default', async () => {
    await act(async () => {
      await i18next.changeLanguage('de');
    });

    renderTopBar();

    const expectedDe = new Date(mockPortfolio.lastUpdated).toLocaleString('de');
    const unlocalized = new Date(mockPortfolio.lastUpdated).toLocaleString();

    expect(screen.getByText(new RegExp(expectedDe.replace(/[.,]/g, '\\$&')))).toBeTruthy();
    // Guard against the pre-fix behavior (toLocaleString() with no locale arg, ignoring i18next).
    if (expectedDe !== unlocalized) {
      expect(screen.queryByText(new RegExp(unlocalized.replace(/[.,]/g, '\\$&')))).toBeNull();
    }
  });
});
