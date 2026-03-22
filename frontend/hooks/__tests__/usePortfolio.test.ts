import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';

// Ensure isTauri() returns false (no Tauri globals in happy-dom)
beforeEach(() => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe('usePortfolio hook (mock/browser path)', () => {
  it('starts in loading state then resolves with mock data', async () => {
    const { usePortfolio, PortfolioProvider } = await import('../../hooks/usePortfolio');
    const { createElement } = await import('react');

    const wrapper = ({ children }: { children: React.ReactNode }) =>
      createElement(PortfolioProvider, null, children);

    const { result } = renderHook(() => usePortfolio(), { wrapper });

    // Initially loading
    expect(result.current.loading).toBe(true);

    // After the 500ms mock delay resolves
    await waitFor(() => expect(result.current.loading).toBe(false), { timeout: 2000 });

    expect(result.current.portfolio).not.toBeNull();
    expect(result.current.holdings.length).toBeGreaterThan(0);
    expect(result.current.error).toBeNull();
  });

  it('loading transitions from true to false', async () => {
    const { usePortfolio, PortfolioProvider } = await import('../../hooks/usePortfolio');
    const { createElement } = await import('react');

    const wrapper = ({ children }: { children: React.ReactNode }) =>
      createElement(PortfolioProvider, null, children);

    const { result } = renderHook(() => usePortfolio(), { wrapper });

    expect(result.current.loading).toBe(true);
    await waitFor(() => expect(result.current.loading).toBe(false), { timeout: 2000 });
  });

  it('addHolding in mock mode creates a holding and adds it to state', async () => {
    const { usePortfolio, PortfolioProvider } = await import('../../hooks/usePortfolio');
    const { createElement } = await import('react');

    const wrapper = ({ children }: { children: React.ReactNode }) =>
      createElement(PortfolioProvider, null, children);

    const { result } = renderHook(() => usePortfolio(), { wrapper });
    await waitFor(() => expect(result.current.loading).toBe(false), { timeout: 2000 });

    const initialCount = result.current.holdings.length;

    await act(async () => {
      await result.current.addHolding({
        symbol: 'TEST',
        name: 'Test Corp',
        assetType: 'stock',
        account: 'taxable',
        quantity: 10,
        costBasis: 100,
        currency: 'USD',
        exchange: 'NYSE',
        targetWeight: 0,
        indicatedAnnualDividend: null,
        indicatedAnnualDividendCurrency: null,
        dividendFrequency: null,
        maturityDate: null,
      });
    });

    expect(result.current.holdings.length).toBe(initialCount + 1);
    expect(result.current.holdings.some((h) => h.symbol === 'TEST')).toBe(true);
  });

  it('deleteHolding removes a holding from state', async () => {
    const { usePortfolio, PortfolioProvider } = await import('../../hooks/usePortfolio');
    const { createElement } = await import('react');

    const wrapper = ({ children }: { children: React.ReactNode }) =>
      createElement(PortfolioProvider, null, children);

    const { result } = renderHook(() => usePortfolio(), { wrapper });
    await waitFor(() => expect(result.current.loading).toBe(false), { timeout: 2000 });

    const holdingsBefore = result.current.holdings;
    expect(holdingsBefore.length).toBeGreaterThan(0);
    const idToDelete = holdingsBefore[0]!.id;

    await act(async () => {
      await result.current.deleteHolding(idToDelete);
    });

    expect(result.current.holdings.some((h) => h.id === idToDelete)).toBe(false);
    expect(result.current.holdings.length).toBe(holdingsBefore.length - 1);
  });

  it('importHoldingsCsv parses CSV and adds holdings in mock mode', async () => {
    const { usePortfolio, PortfolioProvider } = await import('../../hooks/usePortfolio');
    const { createElement } = await import('react');

    const wrapper = ({ children }: { children: React.ReactNode }) =>
      createElement(PortfolioProvider, null, children);

    const { result } = renderHook(() => usePortfolio(), { wrapper });
    await waitFor(() => expect(result.current.loading).toBe(false), { timeout: 2000 });

    const initialCount = result.current.holdings.length;

    const csvContent = [
      'symbol,name,type,account,quantity,cost_basis,currency,exchange,target_weight',
      'GOOG,Alphabet Inc.,stock,taxable,5,120,USD,NASDAQ,0',
    ].join('\n');

    let importResult: Awaited<ReturnType<typeof result.current.importHoldingsCsv>>;
    await act(async () => {
      importResult = await result.current.importHoldingsCsv(csvContent);
    });

    expect(result.current.holdings.length).toBe(initialCount + 1);
    expect(importResult!.imported.length).toBe(1);
    expect(importResult!.totalRows).toBe(1);
  });

  it('exports CSV with correct headers when holdings exist', async () => {
    const { usePortfolio, PortfolioProvider } = await import('../../hooks/usePortfolio');
    const { createElement } = await import('react');

    const wrapper = ({ children }: { children: React.ReactNode }) =>
      createElement(PortfolioProvider, null, children);

    const { result } = renderHook(() => usePortfolio(), { wrapper });
    await waitFor(() => expect(result.current.loading).toBe(false), { timeout: 2000 });

    let csv = '';
    await act(async () => {
      csv = await result.current.exportHoldingsCsv();
    });

    expect(csv).toContain('symbol');
    expect(csv).toContain('quantity');
    expect(csv).toContain('cost_basis');
  });

  it('failedSymbols and triggeredAlertIds start empty', async () => {
    const { usePortfolio, PortfolioProvider } = await import('../../hooks/usePortfolio');
    const { createElement } = await import('react');

    const wrapper = ({ children }: { children: React.ReactNode }) =>
      createElement(PortfolioProvider, null, children);

    const { result } = renderHook(() => usePortfolio(), { wrapper });
    await waitFor(() => expect(result.current.loading).toBe(false), { timeout: 2000 });

    expect(result.current.failedSymbols).toEqual([]);
    expect(result.current.triggeredAlertIds).toEqual([]);
  });
});
