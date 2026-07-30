import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { Dashboard } from '../Dashboard';
import type { PortfolioSnapshot } from '../../types/portfolio';
import { MOCK_SNAPSHOT } from '../../lib/mockData';
import { formatCurrency } from '../../lib/format';

// Initialize i18n (needed by sibling components)
import i18next from '../../lib/i18n';

// Mock ActionCenter and useActionInsights to keep tests focused on Dashboard rendering
vi.mock('../../hooks/useActionInsights', () => ({
  useActionInsights: () => [],
}));
vi.mock('../ActionCenter', () => ({
  ActionCenter: () => null,
}));

beforeEach(async () => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
  // Pin language so English-text assertions don't break if the default locale changes.
  await i18next.changeLanguage('en');
});

function renderDashboard(
  props: { portfolio: PortfolioSnapshot | null; loading: boolean },
  initialEntries: string[] = ['/']
) {
  return render(
    <MemoryRouter initialEntries={initialEntries}>
      <Dashboard {...props} />
    </MemoryRouter>
  );
}

describe('Dashboard component smoke tests', () => {
  it('renders without crashing with no portfolio', () => {
    const { container } = renderDashboard({ portfolio: null, loading: false });
    expect(container).toBeTruthy();
  });

  it('shows empty state message when portfolio is null and not loading', () => {
    renderDashboard({ portfolio: null, loading: false });
    expect(screen.getByText(/add your first holding/i)).toBeTruthy();
  });

  it('shows empty state when portfolio has no holdings', () => {
    const emptyPortfolio: PortfolioSnapshot = {
      ...MOCK_SNAPSHOT,
      holdings: [],
      totalValue: 0,
    };
    renderDashboard({ portfolio: emptyPortfolio, loading: false });
    expect(screen.getByText(/add your first holding/i)).toBeTruthy();
  });

  it('renders portfolio value panel when portfolio has holdings', () => {
    renderDashboard({ portfolio: MOCK_SNAPSHOT as PortfolioSnapshot, loading: false });
    // The portfolio value label is always rendered
    expect(screen.getAllByText(/portfolio value/i).length).toBeGreaterThan(0);
  });

  it('renders without crashing while loading (portfolio=null, loading=true)', () => {
    // loading=true with portfolio=null: the empty-state guard is `!portfolio && !loading`
    // so loading=true should NOT show the empty state; component may show nothing or partial UI
    const { container } = renderDashboard({ portfolio: null, loading: true });
    expect(container).toBeTruthy();
    expect(screen.queryByText(/add your first holding/i)).toBeNull();
  });

  it('shows holdings count when portfolio is loaded', () => {
    renderDashboard({ portfolio: MOCK_SNAPSHOT as PortfolioSnapshot, loading: false });
    // "Holdings" stat label is rendered in the portfolio value panel
    expect(screen.getByText('Holdings')).toBeTruthy();
  });

  it('shows top movers section when portfolio has non-cash holdings', () => {
    renderDashboard({ portfolio: MOCK_SNAPSHOT as PortfolioSnapshot, loading: false });
    expect(screen.getByText(/top movers/i)).toBeTruthy();
  });

  it('shows allocation section', () => {
    renderDashboard({ portfolio: MOCK_SNAPSHOT as PortfolioSnapshot, loading: false });
    expect(screen.getAllByText(/allocation/i).length).toBeGreaterThan(0);
  });
});

describe('Dashboard account filter', () => {
  it('recomputes the portfolio value and holdings count when an account filter is set via URL', () => {
    const snapshot = MOCK_SNAPSHOT as PortfolioSnapshot;
    const tfsaHoldings = snapshot.holdings.filter((h) => h.account === 'tfsa');
    // Sanity check the fixture actually exercises the filter (not all accounts, not zero).
    expect(tfsaHoldings.length).toBeGreaterThan(0);
    expect(tfsaHoldings.length).toBeLessThan(snapshot.holdings.length);

    const expectedTotal = tfsaHoldings.reduce((sum, h) => sum + h.marketValueCad, 0);

    renderDashboard({ portfolio: snapshot, loading: false }, ['/?account=tfsa']);

    // Scope to the Portfolio Value panel — the "By Account" breakdown panel can show
    // the same TFSA subtotal, so an unscoped query would match both.
    const portfolioValuePanel = screen.getByText(/portfolio value/i).parentElement!;
    expect(
      within(portfolioValuePanel).getByText(formatCurrency(expectedTotal, snapshot.baseCurrency))
    ).toBeTruthy();
    expect(
      within(portfolioValuePanel).getByText(
        `${tfsaHoldings.length} position${tfsaHoldings.length !== 1 ? 's' : ''}`
      )
    ).toBeTruthy();
  });

  it('shows an empty state when the filtered account has no holdings', () => {
    const snapshot = MOCK_SNAPSHOT as PortfolioSnapshot;
    // No holding in the fixture uses the 'fhsa' account.
    expect(snapshot.holdings.some((h) => h.account === 'fhsa')).toBe(false);

    renderDashboard({ portfolio: snapshot, loading: false }, ['/?account=fhsa']);

    expect(screen.getByText(/no holdings in this account/i)).toBeTruthy();
  });
});
