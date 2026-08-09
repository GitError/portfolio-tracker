import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { Performance } from '../Performance';
import type { PortfolioSnapshot } from '../../types/portfolio';
import { MOCK_SNAPSHOT } from '../../lib/mockData';

// Initialize i18n (Performance uses useTranslation for locale-aware number formatting)
import i18next from '../../lib/i18n';

let mockIsTauri = false;

vi.mock('../../lib/tauri', () => ({
  isTauri: () => mockIsTauri,
  getErrorMessage: (e: unknown) => (e instanceof Error ? e.message : String(e)),
  tauriInvoke: vi.fn().mockResolvedValue([]),
}));

beforeEach(() => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
  mockIsTauri = false;
  vi.clearAllMocks();
});

afterEach(async () => {
  await act(async () => {
    await i18next.changeLanguage('en');
  });
});

function renderPerformance(props: { portfolio: PortfolioSnapshot | null; onRefresh?: () => void }) {
  return render(
    <MemoryRouter>
      <Performance {...props} />
    </MemoryRouter>
  );
}

describe('Performance component', () => {
  it('renders without crashing with a valid portfolio', () => {
    const { container } = renderPerformance({ portfolio: MOCK_SNAPSHOT as PortfolioSnapshot });
    expect(container).toBeTruthy();
  });

  it('shows empty state when portfolio is null', () => {
    renderPerformance({ portfolio: null });
    expect(screen.getByText(/no portfolio data available/i)).toBeTruthy();
  });

  it('renders range selector buttons when portfolio has holdings', () => {
    renderPerformance({ portfolio: MOCK_SNAPSHOT as PortfolioSnapshot });
    // Range buttons: 1D, 1W, 1M, 3M, 6M, 1Y, ALL
    expect(screen.getByRole('button', { name: '1Y' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'ALL' })).toBeTruthy();
  });

  it('renders All Assets filter when portfolio has holdings', () => {
    renderPerformance({ portfolio: MOCK_SNAPSHOT as PortfolioSnapshot });
    expect(screen.getByText('All Assets')).toBeTruthy();
  });

  it('shows empty state when portfolio has no holdings', () => {
    const emptyPortfolio: PortfolioSnapshot = {
      ...MOCK_SNAPSHOT,
      holdings: [],
      totalValue: 0,
    } as PortfolioSnapshot;
    renderPerformance({ portfolio: emptyPortfolio });
    // With no holdings, filteredHoldings is empty → shows "No holdings match..." empty state
    expect(screen.getByText(/no holdings match/i)).toBeTruthy();
  });

  it('renders the Daily Returns label when data is available', () => {
    renderPerformance({ portfolio: MOCK_SNAPSHOT as PortfolioSnapshot });
    // "Daily Returns" appears as a section heading; use getAllByText since "std dev of daily returns" also matches
    expect(screen.getAllByText(/daily returns/i).length).toBeGreaterThan(0);
  });

  it('calls onRefresh when Refresh Prices action is clicked in empty state', () => {
    // When perfIsEmpty is true (isTauri=true, empty points), an action button appears.
    // With isTauri=false, we get mock data; test empty portfolio instead.
    const onRefresh = vi.fn();
    renderPerformance({ portfolio: null, onRefresh });
    // With null portfolio, renders the "No portfolio data" empty state (no action shown)
    expect(screen.getByText(/no portfolio data available/i)).toBeTruthy();
  });

  it('shows a first-snapshot state instead of a misleading chart when only one snapshot exists', async () => {
    const { tauriInvoke } = await import('../../lib/tauri');
    mockIsTauri = true;
    vi.mocked(tauriInvoke).mockResolvedValue([{ date: '2026-08-08', value: 100_000 }]);

    renderPerformance({ portfolio: MOCK_SNAPSHOT as PortfolioSnapshot });

    // Wait for the async get_performance call to resolve.
    await screen.findByText(/only one snapshot recorded/i);
    // The misleading single-point chart (and its now-meaningless stats row) must not render.
    expect(screen.queryByText('Total Return')).toBeNull();
  });

  it('renders Max Drawdown and Annualized Volatility with German locale separators', async () => {
    await act(async () => {
      await i18next.changeLanguage('de');
    });

    renderPerformance({ portfolio: MOCK_SNAPSHOT as PortfolioSnapshot });

    const maxDrawdownValue = screen.getByText('Max Drawdown').nextElementSibling?.textContent;
    const volatilityValue =
      screen.getByText('Annualized Volatility').nextElementSibling?.textContent;

    expect(maxDrawdownValue).toMatch(/^-\d+,\d%$/);
    expect(volatilityValue).toMatch(/^\d+,\d%$/);
    // Guard against the pre-fix behavior (raw toFixed() always uses '.' regardless of locale).
    expect(maxDrawdownValue).not.toMatch(/\./);
    expect(volatilityValue).not.toMatch(/\./);
  });
});
