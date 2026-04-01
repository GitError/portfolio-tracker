import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

import '../../lib/i18n';
import { TransactionHistory } from '../TransactionHistory';

const mockTransactions = [
  {
    id: 'tx-1',
    holdingId: 'h-1',
    transactionType: 'buy',
    quantity: 10,
    price: 150,
    transactedAt: '2024-01-15T10:00:00Z',
    notes: null,
    createdAt: '2024-01-15T10:00:00Z',
  },
  {
    id: 'tx-2',
    holdingId: 'h-1',
    transactionType: 'sell',
    quantity: 5,
    price: 180,
    transactedAt: '2024-06-01T12:00:00Z',
    notes: null,
    createdAt: '2024-06-01T12:00:00Z',
  },
  {
    id: 'tx-3',
    holdingId: 'h-2',
    transactionType: 'buy',
    quantity: 1,
    price: 30000,
    transactedAt: '2024-03-01T09:00:00Z',
    notes: null,
    createdAt: '2024-03-01T09:00:00Z',
  },
];

const mockHoldings = [
  {
    id: 'h-1',
    symbol: 'AAPL',
    name: 'Apple Inc',
    assetType: 'stock',
    account: 'taxable',
    quantity: 5,
    costBasis: 150,
    currency: 'CAD',
    exchange: 'TSX',
    targetWeight: 0,
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
    indicatedAnnualDividend: null,
    indicatedAnnualDividendCurrency: null,
    dividendFrequency: null,
    maturityDate: null,
    currentPrice: 180,
    currentPriceCad: 180,
    marketValueCad: 900,
    costValueCad: 750,
    gainLoss: 150,
    gainLossPercent: 20,
    weight: 50,
    targetValue: 0,
    targetDeltaValue: 0,
    targetDeltaPercent: 0,
    dailyChangePercent: 0,
    fxStale: false,
  },
  {
    id: 'h-2',
    symbol: 'BTC-USD',
    name: 'Bitcoin',
    assetType: 'crypto',
    account: 'crypto',
    quantity: 1,
    costBasis: 30000,
    currency: 'USD',
    exchange: 'Coinbase',
    targetWeight: 0,
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
    indicatedAnnualDividend: null,
    indicatedAnnualDividendCurrency: null,
    dividendFrequency: null,
    maturityDate: null,
    currentPrice: 35000,
    currentPriceCad: 47250,
    marketValueCad: 47250,
    costValueCad: 40500,
    gainLoss: 6750,
    gainLossPercent: 16.67,
    weight: 50,
    targetValue: 0,
    targetDeltaValue: 0,
    targetDeltaPercent: 0,
    dailyChangePercent: 0,
    fxStale: false,
  },
];

vi.mock('../../lib/tauri', () => ({
  isTauri: () => true,
  tauriInvoke: vi.fn((cmd: string) => {
    if (cmd === 'get_transactions') return Promise.resolve(mockTransactions);
    if (cmd === 'delete_transaction') return Promise.resolve(true);
    return Promise.resolve(null);
  }),
}));

vi.mock('../../hooks/usePortfolio', () => ({
  PortfolioProvider: ({ children }: { children: React.ReactNode }) => children,
  usePortfolio: () => ({
    portfolio: {
      holdings: mockHoldings,
      totalValue: 48150,
      totalCost: 41250,
      totalGainLoss: 6900,
      totalGainLossPercent: 16.73,
      dailyPnl: 0,
      lastUpdated: '2024-01-01T00:00:00Z',
      baseCurrency: 'CAD',
      totalTargetWeight: 0,
      targetCashDelta: 0,
      realizedGains: 0,
      annualDividendIncome: 0,
    },
    loading: false,
    error: null,
    failedSymbols: [],
    triggeredAlertIds: [],
    refreshPrices: vi.fn(),
  }),
}));

beforeEach(() => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
  vi.clearAllMocks();
});

function renderTxHistory() {
  return render(
    <MemoryRouter>
      <TransactionHistory />
    </MemoryRouter>
  );
}

describe('TransactionHistory component', () => {
  it('renders the Transactions heading', async () => {
    renderTxHistory();
    await waitFor(() => screen.getByText('Transactions'));
    expect(screen.getByText('Transactions')).toBeTruthy();
  });

  it('renders a record count badge', async () => {
    renderTxHistory();
    await waitFor(() => screen.getByText('3 records'));
    expect(screen.getByText('3 records')).toBeTruthy();
  });

  it('groups transactions by holding symbol', async () => {
    renderTxHistory();
    // Each symbol appears at least once (in the group header)
    await waitFor(() => screen.getAllByText('AAPL'));
    expect(screen.getAllByText('AAPL').length).toBeGreaterThan(0);
    expect(screen.getAllByText('BTC-USD').length).toBeGreaterThan(0);
  });

  it('renders Add Transaction button', async () => {
    renderTxHistory();
    await waitFor(() => screen.getAllByRole('button', { name: /add transaction/i }));
    expect(screen.getAllByRole('button', { name: /add transaction/i }).length).toBeGreaterThan(0);
  });

  it('shows transaction type badges (buy/sell)', async () => {
    renderTxHistory();
    // buy badges are uppercase text in a styled span
    await waitFor(() => screen.getAllByText('buy'));
    expect(screen.getAllByText('buy').length).toBeGreaterThan(0);
  });

  it('collapses a holding group when header is clicked', async () => {
    renderTxHistory();

    // AAPL has 2 transactions; find the section header by that label
    await waitFor(() => screen.getByText('2 transactions ▼'));

    // The sell badge is visible before collapse
    expect(screen.getByText('sell')).toBeTruthy();

    // Click the section header button (which contains "2 transactions ▼")
    const header = screen.getByText('2 transactions ▼').closest('button');
    expect(header).toBeTruthy();
    if (header) fireEvent.click(header);

    // After collapsing the AAPL group, the sell badge should be gone
    await waitFor(() => {
      expect(screen.queryByText('sell')).toBeNull();
    });
  });

  it('shows delete button for each transaction', async () => {
    renderTxHistory();
    await waitFor(() => screen.getAllByRole('button', { name: /delete/i }));
    const deleteButtons = screen.getAllByRole('button', { name: /delete/i });
    // 3 transactions = 3 delete buttons
    expect(deleteButtons.length).toBe(3);
  });

  it('shows confirm/cancel buttons after clicking delete', async () => {
    renderTxHistory();
    await waitFor(() => screen.getAllByRole('button', { name: /delete/i }));
    const [firstDelete] = screen.getAllByRole('button', { name: /delete/i });
    if (firstDelete) fireEvent.click(firstDelete);

    await waitFor(() => screen.getByText('Confirm'));
    expect(screen.getByText('Confirm')).toBeTruthy();
    expect(screen.getByText('Cancel')).toBeTruthy();
  });

  it('cancels delete when Cancel is clicked', async () => {
    renderTxHistory();
    await waitFor(() => screen.getAllByRole('button', { name: /delete/i }));
    const [firstDelete] = screen.getAllByRole('button', { name: /delete/i });
    if (firstDelete) fireEvent.click(firstDelete);

    await waitFor(() => screen.getByText('Cancel'));
    fireEvent.click(screen.getByText('Cancel'));

    await waitFor(() => {
      expect(screen.queryByText('Confirm')).toBeNull();
    });
  });
});
