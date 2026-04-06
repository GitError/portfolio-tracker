import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { AddTransactionModal } from '../AddTransactionModal';
import { ToastProvider } from '../ui/Toast';
import type { Holding } from '../../types/portfolio';

const mockTauriInvoke = vi.fn();

vi.mock('../../lib/tauri', () => ({
  isTauri: () => true,
  tauriInvoke: (...args: unknown[]) => mockTauriInvoke(...args),
}));

const MOCK_HOLDING: Holding = {
  id: 'h-1',
  symbol: 'AAPL',
  name: 'Apple Inc.',
  assetType: 'stock',
  account: 'taxable',
  quantity: 10,
  costBasis: 150,
  currency: 'USD',
  exchange: 'NASDAQ',
  targetWeight: 0,
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
  indicatedAnnualDividend: null,
  indicatedAnnualDividendCurrency: null,
  dividendFrequency: null,
  maturityDate: null,
};

function renderModal(props: Partial<React.ComponentProps<typeof AddTransactionModal>> = {}) {
  const defaults = {
    holding: MOCK_HOLDING,
    isOpen: true,
    onClose: vi.fn(),
    onSaved: vi.fn(),
  };
  return render(
    <ToastProvider>
      <AddTransactionModal {...defaults} {...props} />
    </ToastProvider>
  );
}

beforeEach(() => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
  vi.clearAllMocks();
  mockTauriInvoke.mockResolvedValue({
    id: 'tx-1',
    holdingId: 'h-1',
    transactionType: 'buy',
    quantity: 10,
    price: 150,
    transactedAt: new Date().toISOString(),
    notes: null,
    createdAt: new Date().toISOString(),
  });
});

describe('AddTransactionModal', () => {
  it('renders without crashing', () => {
    const { container } = renderModal();
    expect(container).toBeTruthy();
  });

  it('renders nothing when isOpen=false', () => {
    renderModal({ isOpen: false });
    expect(screen.queryByText('Log Transaction')).toBeNull();
  });

  it('renders the modal heading when isOpen=true', () => {
    renderModal();
    expect(screen.getByText('Log Transaction')).toBeTruthy();
  });

  it('shows the holding symbol in the modal subtitle', () => {
    renderModal();
    expect(screen.getByText(/AAPL/)).toBeTruthy();
  });

  it('shows Buy and Sell type buttons', () => {
    renderModal();
    expect(screen.getByRole('button', { name: /buy/i })).toBeTruthy();
    expect(screen.getByRole('button', { name: /sell/i })).toBeTruthy();
  });

  it('Cancel button calls onClose', () => {
    const onClose = vi.fn();
    renderModal({ onClose });
    fireEvent.click(screen.getByRole('button', { name: /cancel/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('shows Quantity and Price fields', () => {
    renderModal();
    const inputs = screen.getAllByRole('spinbutton');
    expect(inputs.length).toBeGreaterThanOrEqual(2);
  });

  it('shows Save button', () => {
    renderModal();
    expect(screen.getByRole('button', { name: /save/i })).toBeTruthy();
  });

  it('submit with empty quantity shows validation error', async () => {
    renderModal();
    const form = screen.getByRole('button', { name: /save/i }).closest('form');
    expect(form).toBeTruthy();
    if (form) {
      fireEvent.submit(form);
    }
    await waitFor(() => {
      expect(screen.getByText(/quantity must be greater than 0/i)).toBeTruthy();
    });
  });

  it('valid form triggers tauriInvoke add_transaction', async () => {
    const onSaved = vi.fn();
    const onClose = vi.fn();
    renderModal({ onSaved, onClose });

    const [quantityInput, priceInput] = screen.getAllByRole('spinbutton');
    if (quantityInput) fireEvent.change(quantityInput, { target: { value: '10' } });
    if (priceInput) fireEvent.change(priceInput, { target: { value: '150' } });

    const form = screen.getByRole('button', { name: /save/i }).closest('form');
    if (form) fireEvent.submit(form);

    await waitFor(() => {
      expect(mockTauriInvoke).toHaveBeenCalledWith(
        'add_transaction',
        expect.objectContaining({
          input: expect.objectContaining({
            holdingId: 'h-1',
            transactionType: 'buy',
            quantity: 10,
            price: 150,
          }),
        })
      );
    });

    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledTimes(1);
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  it('shows error when tauriInvoke rejects', async () => {
    mockTauriInvoke.mockRejectedValue(new Error('Network error'));
    renderModal();

    const [quantityInput, priceInput] = screen.getAllByRole('spinbutton');
    if (quantityInput) fireEvent.change(quantityInput, { target: { value: '5' } });
    if (priceInput) fireEvent.change(priceInput, { target: { value: '100' } });

    const form = screen.getByRole('button', { name: /save/i }).closest('form');
    if (form) fireEvent.submit(form);

    await waitFor(() => {
      expect(screen.getByText(/Network error/i)).toBeTruthy();
    });
  });
});
