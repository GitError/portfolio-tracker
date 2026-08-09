import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, act, fireEvent } from '@testing-library/react';
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

describe('TopBar failed-symbols banner dismiss (#786)', () => {
  it('shows Retry and Dismiss when a price refresh fails', () => {
    renderTopBar({ isOffline: false, failedSymbols: ['AAPL', 'MSFT'] });

    expect(screen.getByText(/Price refresh failed for: AAPL, MSFT/)).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Retry' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Dismiss' })).toBeTruthy();
  });

  it('dismiss hides the banner without calling onRefresh or clearing failedSymbols', () => {
    const onRefresh = vi.fn();
    renderTopBar({ isOffline: false, failedSymbols: ['AAPL'], onRefresh });

    fireEvent.click(screen.getByRole('button', { name: 'Dismiss' }));

    expect(screen.queryByText(/Price refresh failed for:/)).toBeNull();
    expect(onRefresh).not.toHaveBeenCalled();
  });

  it('retry still calls onRefresh and leaves the banner in place', () => {
    const onRefresh = vi.fn();
    renderTopBar({ isOffline: false, failedSymbols: ['AAPL'], onRefresh });

    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));

    expect(onRefresh).toHaveBeenCalledTimes(1);
    expect(screen.getByText(/Price refresh failed for:/)).toBeTruthy();
  });

  it('a new failure (different symbols) reopens the banner after a dismiss', () => {
    const onRefresh = vi.fn();
    const { rerender } = renderTopBar({ isOffline: false, failedSymbols: ['AAPL'], onRefresh });

    fireEvent.click(screen.getByRole('button', { name: 'Dismiss' }));
    expect(screen.queryByText(/Price refresh failed for:/)).toBeNull();

    rerender(
      <MemoryRouter>
        <TopBar
          portfolio={mockPortfolio}
          loading={false}
          isOffline={false}
          onRefresh={onRefresh}
          baseCurrency="CAD"
          onBaseCurrencyChange={vi.fn()}
          failedSymbols={['GOOG']}
        />
      </MemoryRouter>
    );

    expect(screen.getByText(/Price refresh failed for: GOOG/)).toBeTruthy();
  });

  it('the Dismiss control is keyboard-accessible', () => {
    renderTopBar({ isOffline: false, failedSymbols: ['AAPL'] });

    const dismissButton = screen.getByRole('button', { name: 'Dismiss' });
    expect(dismissButton.tagName).toBe('BUTTON');
    dismissButton.focus();
    expect(document.activeElement).toBe(dismissButton);
  });
});
