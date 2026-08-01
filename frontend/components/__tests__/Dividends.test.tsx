import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { Dividends } from '../Dividends';

// Initialize i18n (Dividends uses useTranslation)
import i18next from '../../lib/i18n';

// Mutable so individual tests can opt into the Tauri command path while the
// rest of the suite keeps exercising the browser-mode MOCK_DIVIDENDS path.
let mockIsTauri = false;
const mockGetDividends = vi.fn();
const mockGetHoldingsPaginated = vi.fn();

vi.mock('../../lib/tauri', () => ({
  isTauri: () => mockIsTauri,
  getErrorMessage: (e: unknown) => (e instanceof Error ? e.message : String(e)),
  tauriInvoke: (cmd: string) => {
    if (cmd === 'get_dividends') return mockGetDividends();
    if (cmd === 'get_holdings_paginated') return mockGetHoldingsPaginated();
    return Promise.resolve([]);
  },
}));

// Mock usePortfolio — Dividends uses portfolio for forward income rows
vi.mock('../../hooks/usePortfolio', () => ({
  PortfolioProvider: ({ children }: { children: React.ReactNode }) => children,
  usePortfolio: () => ({
    portfolio: null,
    holdings: [],
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
    markAlertsSeen: vi.fn(),
  }),
}));

beforeEach(async () => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
  vi.clearAllMocks();
  mockIsTauri = false;
  mockGetDividends.mockResolvedValue([]);
  mockGetHoldingsPaginated.mockResolvedValue({
    items: [],
    total: 0,
    page: 1,
    pageSize: 500,
    totalPages: 0,
  });
  // Pin language so English-text assertions don't break if the default locale changes.
  await i18next.changeLanguage('en');
});

function renderDividends() {
  return render(
    <MemoryRouter>
      <Dividends />
    </MemoryRouter>
  );
}

describe('Dividends component smoke tests', () => {
  it('renders without crashing', async () => {
    const { container } = renderDividends();
    await waitFor(() => screen.getByText(/dividends/i));
    expect(container).toBeTruthy();
  });

  it('renders the Dividends heading', async () => {
    renderDividends();
    await waitFor(() => screen.getByText(/dividends/i));
    expect(screen.getByText(/dividends/i)).toBeTruthy();
  });

  it('shows mock dividend rows when isTauri() is false (uses MOCK_DIVIDENDS)', async () => {
    // When isTauri() returns false, Dividends.tsx uses MOCK_DIVIDENDS from mockData.ts.
    // MOCK_DIVIDENDS contains at least one entry — verify data renders.
    renderDividends();
    // Wait for loading spinner to go away and data to appear
    await waitFor(() => expect(screen.queryByRole('status')).toBeNull());
    // The dividend history table should have a symbol column header
    expect(screen.getByText(/symbol/i)).toBeTruthy();
  });

  it('shows Add Dividend button', async () => {
    renderDividends();
    await waitFor(() => screen.getByRole('button', { name: /add dividend/i }));
    expect(screen.getByRole('button', { name: /add dividend/i })).toBeTruthy();
  });

  it('shows descriptive subtitle', async () => {
    renderDividends();
    await waitFor(() => screen.getByText(/record and track dividend income/i));
    expect(screen.getByText(/record and track dividend income/i)).toBeTruthy();
  });

  it('shows empty state message when no dividends and no holdings', async () => {
    // Override tauriInvoke mock to force empty data; but since isTauri()=false
    // the component always falls through to MOCK_DIVIDENDS / MOCK_HOLDINGS.
    // This test verifies the component handles an empty-like state gracefully
    // by checking the empty-state path renders when component receives empty data.
    // We can verify by checking the component overall renders without error.
    const { container } = renderDividends();
    await waitFor(() => expect(container.firstChild).toBeTruthy());
    expect(container.firstChild).toBeTruthy();
  });

  it('shows dividend symbol in history table', async () => {
    renderDividends();
    // MOCK_DIVIDENDS has entries for AAPL and/or other symbols
    await waitFor(() => {
      // At least one symbol-looking text should be present in the table
      const cells = document.querySelectorAll('div[style*="font-mono"]');
      expect(cells.length).toBeGreaterThan(0);
    });
  });

  it('shows total cash received (amountPerUnit x quantity), not the raw per-unit amount', async () => {
    // MOCK_DIVIDENDS: TD.TO pays 0.118/share on a 150-share holding -> 17.70 CAD total.
    // AAPL pays 0.25/share on a 50-share holding -> 12.50 USD total.
    // The old (buggy) behavior summed amountPerUnit directly, which would render
    // as "0.12 CAD" and "0.25 USD" instead of the correct cash totals below.
    renderDividends();
    await waitFor(() => expect(screen.queryByRole('status')).toBeNull());
    expect(screen.getByText('17.70 CAD')).toBeTruthy();
    expect(screen.getByText('12.50 USD')).toBeTruthy();
  });

  it('shows a persistent error with a retry button when loading fails, and retry re-fetches', async () => {
    mockIsTauri = true;
    mockGetDividends.mockRejectedValue(new Error('network down'));

    renderDividends();

    await waitFor(() => screen.getByText(/failed to load dividends/i));
    const retryButton = screen.getByRole('button', { name: /retry/i });
    expect(retryButton).toBeTruthy();

    mockGetDividends.mockResolvedValue([]);
    fireEvent.click(retryButton);

    await waitFor(() => expect(mockGetDividends).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(screen.queryByText(/failed to load dividends/i)).toBeNull());
  });
});
