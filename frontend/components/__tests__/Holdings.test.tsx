import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { Holdings } from '../Holdings';
import type { HoldingWithPrice, PortfolioSnapshot } from '../../types/portfolio';
import { MOCK_SNAPSHOT } from '../../lib/mockData';

// Initialize i18n (Holdings uses useTranslation)
import '../../lib/i18n';

// Shared mock state — reassigned per test
let mockHoldings: HoldingWithPrice[] = [];
let mockPortfolio: PortfolioSnapshot | null = null;

vi.mock('../../hooks/usePortfolio', () => ({
  PortfolioProvider: ({ children }: { children: React.ReactNode }) => children,
  usePortfolio: () => ({
    portfolio: mockPortfolio,
    holdings: mockHoldings,
    loading: false,
    error: null,
    failedSymbols: [],
    triggeredAlertIds: [],
    alertRefreshErrors: [],
    refreshPrices: vi.fn(),
    addHolding: vi.fn().mockResolvedValue({}),
    updateHolding: vi.fn().mockResolvedValue({}),
    deleteHolding: vi.fn().mockResolvedValue(undefined),
    importHoldingsCsv: vi.fn().mockResolvedValue({ imported: [], skipped: [], totalRows: 0 }),
    previewImportCsv: vi.fn().mockResolvedValue({ rows: [], readyCount: 0, skipCount: 0 }),
    exportHoldingsCsv: vi.fn().mockResolvedValue(''),
  }),
}));

beforeEach(() => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
  mockHoldings = [];
  mockPortfolio = null;
});

function renderHoldings() {
  return render(
    <MemoryRouter>
      <Holdings />
    </MemoryRouter>
  );
}

describe('Holdings component smoke tests', () => {
  it('renders without crashing', () => {
    const { container } = renderHoldings();
    expect(container).toBeTruthy();
  });

  it('shows empty state message when there are no holdings', () => {
    renderHoldings();
    expect(screen.getByText(/no positions/i)).toBeTruthy();
  });

  it('shows the "Add Holding" action in the empty state', () => {
    renderHoldings();
    // EmptyState renders an action button with "+ Add Holding"
    expect(screen.getAllByText(/add holding/i).length).toBeGreaterThan(0);
  });

  it('renders the Add Holding button in the toolbar', () => {
    renderHoldings();
    // There's a toolbar button for adding a holding even when empty
    const buttons = screen.getAllByRole('button');
    expect(buttons.length).toBeGreaterThan(0);
  });

  it('renders column headers when holdings are present', () => {
    mockHoldings = MOCK_SNAPSHOT.holdings as HoldingWithPrice[];
    mockPortfolio = MOCK_SNAPSHOT as PortfolioSnapshot;
    renderHoldings();
    // Symbol column header should be present
    expect(screen.getByText(/symbol/i)).toBeTruthy();
  });

  it('renders holding rows when holdings are present', () => {
    mockHoldings = MOCK_SNAPSHOT.holdings as HoldingWithPrice[];
    mockPortfolio = MOCK_SNAPSHOT as PortfolioSnapshot;
    renderHoldings();
    // AAPL is in the mock holdings
    expect(screen.getByText('AAPL')).toBeTruthy();
  });

  it('renders multiple holdings in the table', () => {
    mockHoldings = MOCK_SNAPSHOT.holdings as HoldingWithPrice[];
    mockPortfolio = MOCK_SNAPSHOT as PortfolioSnapshot;
    renderHoldings();
    // Both AAPL and MSFT should appear
    expect(screen.getByText('AAPL')).toBeTruthy();
    expect(screen.getByText('MSFT')).toBeTruthy();
  });

  it('does not show empty state when holdings are present', () => {
    mockHoldings = MOCK_SNAPSHOT.holdings as HoldingWithPrice[];
    mockPortfolio = MOCK_SNAPSHOT as PortfolioSnapshot;
    renderHoldings();
    expect(screen.queryByText(/no positions/i)).toBeNull();
  });
});
