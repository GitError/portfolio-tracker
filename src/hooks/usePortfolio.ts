import {
  createContext,
  createElement,
  useState,
  useEffect,
  useCallback,
  useContext,
  type ReactNode,
} from 'react';
import type {
  AccountType,
  Holding,
  HoldingInput,
  ImportResult,
  PortfolioSnapshot,
  PreviewImportResult,
} from '../types/portfolio';
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
  importHoldingsCsv: (csvContent: string) => Promise<ImportResult>;
  previewImportCsv: (csvContent: string) => Promise<PreviewImportResult>;
  exportHoldingsCsv: () => Promise<string>;
}

const PortfolioContext = createContext<UsePortfolioReturn | null>(null);

function parseCSVLine(line: string, delimiter: string): string[] {
  const result: string[] = [];
  let current = '';
  let inQuotes = false;
  for (const char of line) {
    if (char === '"') {
      inQuotes = !inQuotes;
    } else if (char === delimiter && !inQuotes) {
      result.push(current.trim());
      current = '';
    } else {
      current += char;
    }
  }
  result.push(current.trim());
  return result;
}

function parseMockCsv(csvContent: string): HoldingInput[] {
  const lines = csvContent
    .trim()
    .replace(/^\uFEFF/, '')
    .split(/\r?\n/)
    .filter((line) => line.trim().length > 0);

  if (lines.length < 2) return [];

  const rawHeader = lines[0];
  const delimiter = rawHeader.includes(';') && !rawHeader.includes(',') ? ';' : ',';
  const header = parseCSVLine(rawHeader, delimiter).map((field) => field.toLowerCase());
  const columnIndex = (field: string) => header.indexOf(field);

  return lines.slice(1).map((line) => {
    const cells = parseCSVLine(line, delimiter);
    const assetType = cells[columnIndex('type')] as HoldingInput['assetType'];
    const currency = cells[columnIndex('currency')].toUpperCase();
    const rawSymbol = cells[columnIndex('symbol')];
    const rawAccount = cells[columnIndex('account')]?.toLowerCase() as AccountType | undefined;

    return {
      symbol: assetType === 'cash' ? rawSymbol || `${currency}-CASH` : rawSymbol.toUpperCase(),
      name: cells[columnIndex('name')] || (assetType === 'cash' ? `${currency} Cash` : rawSymbol),
      assetType,
      account: rawAccount || (assetType === 'cash' ? 'cash' : 'taxable'),
      quantity: Number(cells[columnIndex('quantity')]),
      costBasis: Number(cells[columnIndex('cost_basis')]),
      currency,
      exchange: (cells[columnIndex('exchange')] ?? '').toUpperCase(),
      targetWeight: Number(cells[columnIndex('target_weight')]) || 0,
    };
  });
}

function usePortfolioState(): UsePortfolioReturn {
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

  const importHoldingsCsv = useCallback(
    async (csvContent: string): Promise<ImportResult> => {
      if (isTauri()) {
        const result = await tauriInvoke<ImportResult>('import_holdings_csv', { csvContent });
        await loadPortfolio();
        return result;
      }

      const now = new Date().toISOString();
      const imported = parseMockCsv(csvContent).map((input, index) => ({
        id: `${Date.now()}-${index}`,
        ...input,
        createdAt: now,
        updatedAt: now,
      }));
      setHoldings((prev) => [...prev, ...imported]);
      return { imported, skipped: [], totalRows: imported.length };
    },
    [loadPortfolio]
  );

  const previewImportCsv = useCallback(async (csvContent: string): Promise<PreviewImportResult> => {
    if (isTauri()) {
      return tauriInvoke<PreviewImportResult>('preview_import_csv', { csvContent });
    }
    // Browser mock: parse the CSV and return a basic preview
    const imported = parseMockCsv(csvContent);
    return {
      rows: imported.map((input, index) => ({
        row: index + 2,
        originalSymbol: input.symbol,
        resolvedSymbol: input.symbol,
        name: input.name,
        assetType: input.assetType,
        currency: input.currency,
        exchange: '',
        quantity: input.quantity,
        costBasis: input.costBasis,
        targetWeight: input.targetWeight,
        status: 'ready',
      })),
      readyCount: imported.length,
      skipCount: 0,
    };
  }, []);

  const exportHoldingsCsv = useCallback(async (): Promise<string> => {
    if (isTauri()) {
      return tauriInvoke<string>('export_holdings_csv');
    }

    const rows = [
      [
        'symbol',
        'name',
        'type',
        'account',
        'quantity',
        'cost_basis',
        'currency',
        'exchange',
        'target_weight',
      ],
      ...holdings.map((holding) => [
        holding.symbol,
        holding.name,
        holding.assetType,
        holding.account,
        String(holding.quantity),
        String(holding.costBasis),
        holding.currency,
        holding.exchange,
        String(holding.targetWeight),
      ]),
    ];
    return rows.map((row) => row.join(',')).join('\n');
  }, [holdings]);

  return {
    portfolio,
    holdings,
    loading,
    error,
    refreshPrices,
    addHolding,
    updateHolding,
    deleteHolding,
    importHoldingsCsv,
    previewImportCsv,
    exportHoldingsCsv,
  };
}

export function PortfolioProvider({ children }: { children: ReactNode }) {
  const value = usePortfolioState();
  return createElement(PortfolioContext.Provider, { value }, children);
}

export function usePortfolio(): UsePortfolioReturn {
  const context = useContext(PortfolioContext);
  if (!context) {
    throw new Error('usePortfolio must be used within a PortfolioProvider');
  }
  return context;
}
