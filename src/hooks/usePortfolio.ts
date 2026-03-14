import { useState, useEffect, useCallback } from 'react';
import type { Holding, HoldingInput, PortfolioSnapshot } from '../types/portfolio';
import { MOCK_SNAPSHOT, MOCK_HOLDINGS } from '../lib/mockData';

// Tauri v2 always sets window.__TAURI_INTERNALS__ inside the webview.
// window.__TAURI__ is only present when app.withGlobalTauri is true — don't use it.
const isTauri = (): boolean => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

export interface UsePortfolioReturn {
  portfolio: PortfolioSnapshot | null;
  holdings: Holding[];
  loading: boolean;
  error: string | null;
  refreshPrices: () => Promise<void>;
  addHolding: (input: HoldingInput) => Promise<Holding>;
  updateHolding: (holding: Holding) => Promise<Holding>;
  deleteHolding: (id: string) => Promise<void>;
}

export function usePortfolio(): UsePortfolioReturn {
  const [portfolio, setPortfolio] = useState<PortfolioSnapshot | null>(null);
  const [holdings, setHoldings] = useState<Holding[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadPortfolio = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      if (isTauri()) {
        const [snap, rawHoldings] = await Promise.all([
          tauriInvoke<PortfolioSnapshot>('get_portfolio'),
          tauriInvoke<Holding[]>('get_holdings'),
        ]);
        setPortfolio(snap);
        setHoldings(rawHoldings);
      } else {
        await new Promise((r) => setTimeout(r, 500));
        setPortfolio(MOCK_SNAPSHOT);
        setHoldings(MOCK_HOLDINGS);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadPortfolio();
  }, [loadPortfolio]);

  const refreshPrices = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      if (isTauri()) {
        await tauriInvoke('refresh_prices');
        await loadPortfolio();
      } else {
        await new Promise((r) => setTimeout(r, 800));
        setPortfolio({ ...MOCK_SNAPSHOT, lastUpdated: new Date().toISOString() });
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [loadPortfolio]);

  const addHolding = useCallback(
    async (input: HoldingInput): Promise<Holding> => {
      if (isTauri()) {
        const created = await tauriInvoke<Holding>('add_holding', { holding: input });
        await loadPortfolio();
        return created;
      }
      // Mock: create a fake holding
      const mock: Holding = {
        id: String(Date.now()),
        ...input,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      setHoldings((prev) => [...prev, mock]);
      return mock;
    },
    [loadPortfolio]
  );

  const updateHolding = useCallback(
    async (holding: Holding): Promise<Holding> => {
      if (isTauri()) {
        const updated = await tauriInvoke<Holding>('update_holding', { holding });
        await loadPortfolio();
        return updated;
      }
      const updated = { ...holding, updatedAt: new Date().toISOString() };
      setHoldings((prev) => prev.map((h) => (h.id === holding.id ? updated : h)));
      return updated;
    },
    [loadPortfolio]
  );

  const deleteHolding = useCallback(
    async (id: string): Promise<void> => {
      if (isTauri()) {
        await tauriInvoke('delete_holding', { id });
        await loadPortfolio();
        return;
      }
      setHoldings((prev) => prev.filter((h) => h.id !== id));
    },
    [loadPortfolio]
  );

  return {
    portfolio,
    holdings,
    loading,
    error,
    refreshPrices,
    addHolding,
    updateHolding,
    deleteHolding,
  };
}
