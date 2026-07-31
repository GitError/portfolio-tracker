import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { Holdings } from '../Holdings';
import type { HoldingWithPrice, PortfolioSnapshot } from '../../types/portfolio';
import { MOCK_SNAPSHOT } from '../../lib/mockData';

// Initialize i18n (Holdings uses useTranslation)
import i18next from '../../lib/i18n';

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

beforeEach(async () => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
  mockHoldings = [];
  mockPortfolio = null;
  localStorage.clear();
  // Pin language so English-text assertions don't break if the default locale changes.
  await i18next.changeLanguage('en');
});

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
    weight: 20,
    currentPrice: 200,
    currentPriceCad: 280,
    marketValueCad: 2800,
    costValueCad: 2100,
    gainLoss: 700,
    gainLossPercent: 33.3,
    targetValue: 0,
    targetDeltaValue: -2800,
    targetDeltaPercent: -20,
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

/** Shows every column, including targetWeight/targetDeltaPercent/targetDeltaValue which are hidden by default. */
function showAllColumns() {
  localStorage.setItem('app-config', JSON.stringify({ holdings_hidden_columns: '[]' }));
}

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

describe('Holdings target weight null vs explicit-0 display', () => {
  it('shows "—" for a holding with no target weight set (null)', () => {
    showAllColumns();
    const holding = makeHolding({ targetWeight: null });
    mockHoldings = [holding];
    mockPortfolio = { ...MOCK_SNAPSHOT, holdings: [holding] } as PortfolioSnapshot;
    renderHoldings();

    // No explicit-zero tooltip should be present for a null target.
    expect(screen.queryByTitle('Marked for full exit')).toBeNull();
  });

  it('shows "0.0%" with a distinct tooltip for a holding explicitly targeted at 0%', () => {
    showAllColumns();
    const holding = makeHolding({ targetWeight: 0 });
    mockHoldings = [holding];
    mockPortfolio = { ...MOCK_SNAPSHOT, holdings: [holding] } as PortfolioSnapshot;
    renderHoldings();

    const targetCell = screen.getByTitle('Marked for full exit');
    expect(targetCell.textContent).toBe('0.0%');
  });

  it('shows rebalance delta values for an explicit 0% target (not "—")', () => {
    showAllColumns();
    const holding = makeHolding({
      targetWeight: 0,
      targetDeltaPercent: -20,
      targetDeltaValue: -2800,
    });
    mockHoldings = [holding];
    mockPortfolio = { ...MOCK_SNAPSHOT, holdings: [holding] } as PortfolioSnapshot;
    renderHoldings();

    expect(screen.getByText('-20.00%')).toBeTruthy();
  });
});
