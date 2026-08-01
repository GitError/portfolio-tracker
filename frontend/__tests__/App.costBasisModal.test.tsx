import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import App from '../App';

// Isolated from App.test.tsx so we can force a Tauri-mode tauriInvoke failure/success
// without affecting the other App tests' browser-mode assumptions.
const tauriInvokeMock = vi.fn();
vi.mock('../lib/tauri', async () => {
  const actual = await vi.importActual<typeof import('../lib/tauri')>('../lib/tauri');
  return {
    ...actual,
    tauriInvoke: (...args: unknown[]) =>
      (tauriInvokeMock as unknown as (...args: unknown[]) => unknown)(...args),
  };
});

function basePortfolioHookValue(overrides: Record<string, unknown> = {}) {
  return {
    portfolio: { requiresCostBasisSelection: true, baseCurrency: 'CAD', holdings: [] },
    loading: false,
    isRefreshing: false,
    isOffline: false,
    error: null,
    failedSymbols: [],
    triggeredAlertIds: [],
    unseenTriggeredCount: 0,
    refreshPrices: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}

const mockUsePortfolio = vi.fn();
vi.mock('../hooks/usePortfolio', () => ({
  PortfolioProvider: ({ children }: { children: React.ReactNode }) => children,
  usePortfolio: () => mockUsePortfolio(),
}));

beforeEach(() => {
  tauriInvokeMock.mockReset();
  mockUsePortfolio.mockReset();
});

describe('Cost-basis modal error handling (#703)', () => {
  it('shows an error toast and keeps the modal open when persistence fails', async () => {
    mockUsePortfolio.mockReturnValue(basePortfolioHookValue());
    tauriInvokeMock.mockRejectedValueOnce(new Error('disk full'));

    render(<App />);

    const avcoButton = await screen.findByText('AVCO (Average Cost)');
    fireEvent.click(avcoButton);

    await waitFor(() => {
      expect(screen.getByText('disk full')).toBeTruthy();
    });
    // Modal must remain open so the user can retry — it never got a successful save.
    expect(screen.getByText('Choose Cost-Basis Method')).toBeTruthy();
  });

  it('closes the modal and refreshes prices on success', async () => {
    const refreshPrices = vi.fn().mockResolvedValue(undefined);
    mockUsePortfolio.mockReturnValue(basePortfolioHookValue({ refreshPrices }));
    tauriInvokeMock.mockResolvedValueOnce(undefined);

    render(<App />);

    const fifoButton = await screen.findByText('FIFO');
    fireEvent.click(fifoButton);

    await waitFor(() => {
      expect(screen.queryByText('Choose Cost-Basis Method')).toBeNull();
    });
    expect(refreshPrices).toHaveBeenCalled();
  });
});
