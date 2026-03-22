import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { Dashboard } from '../Dashboard';
import type { PortfolioSnapshot } from '../../types/portfolio';
import { MOCK_SNAPSHOT } from '../../lib/mockData';

// Initialize i18n (needed by sibling components)
import '../../lib/i18n';

// Mock ActionCenter and useActionInsights to keep tests focused on Dashboard rendering
vi.mock('../../hooks/useActionInsights', () => ({
  useActionInsights: () => [],
}));
vi.mock('../ActionCenter', () => ({
  ActionCenter: () => null,
}));

beforeEach(() => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
});

function renderDashboard(props: { portfolio: PortfolioSnapshot | null; loading: boolean }) {
  return render(
    <MemoryRouter>
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
