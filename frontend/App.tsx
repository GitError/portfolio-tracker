import { useCallback, useEffect, useRef, useState } from 'react';
import { BrowserRouter, Routes, Route, useNavigate } from 'react-router-dom';
import { tauriInvoke } from './lib/tauri';
import { Layout } from './components/Layout';
import { Dashboard } from './components/Dashboard';
import { Holdings } from './components/Holdings';
import { Performance } from './components/Performance';
import { StressTest } from './components/StressTest';
import { Rebalance } from './components/Rebalance';
import { Alerts } from './components/Alerts';
import { Dividends } from './components/Dividends';
import { Settings } from './components/Settings';
import { TransactionHistory } from './components/TransactionHistory';
import { Analytics } from './components/Analytics';
import { Help } from './components/Help';
import { ToastProvider } from './components/ui/Toast';
import { useToast } from './components/ui/Toast';
import { KeyboardShortcutsOverlay } from './components/ui/KeyboardShortcutsOverlay';
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts';
import { PortfolioProvider, usePortfolio } from './hooks/usePortfolio';
import { useConfig } from './hooks/useConfig';
import { useTheme } from './hooks/useTheme';
import { useAutoRefresh } from './hooks/useAutoRefresh';
import { CurrencyContext } from './lib/currencyContext';
import { formatCompact } from './lib/format';

function AppRoutes() {
  // Initialize theme on mount — applies data-theme to <html> and reacts to OS changes
  useTheme();

  const {
    portfolio,
    loading,
    isRefreshing,
    isOffline,
    error,
    failedSymbols,
    triggeredAlertIds,
    unseenTriggeredCount,
    refreshPrices,
  } = usePortfolio();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { value: baseCurrency, setValue: setBaseCurrency } = useConfig('base_currency', 'CAD');
  const [shortcutsHelpOpen, setShortcutsHelpOpen] = useState(false);

  const handleRefresh = useCallback(async (): Promise<void> => {
    await refreshPrices();
  }, [refreshPrices]);

  const { countdown } = useAutoRefresh({
    onRefresh: handleRefresh,
  });

  const [currencyChanging, setCurrencyChanging] = useState(false);

  // Refs for imperative handles registered by Holdings
  const openAddHoldingRef = useRef<(() => void) | null>(null);
  const exportCsvRef = useRef<(() => void) | null>(null);

  // When base currency changes, re-fetch prices so conversions update immediately (#98)
  const handleBaseCurrencyChange = useCallback(
    async (currency: string) => {
      setBaseCurrency(currency);
      setCurrencyChanging(true);
      try {
        await refreshPrices();
      } finally {
        setCurrencyChanging(false);
      }
    },
    [setBaseCurrency, refreshPrices]
  );

  const handleOpenAddHolding = useCallback(() => {
    navigate('/holdings');
    setTimeout(() => {
      openAddHoldingRef.current?.();
    }, 50);
  }, [navigate]);

  const handleExportCsv = useCallback(() => {
    exportCsvRef.current?.();
  }, []);

  const handleToggleHelp = useCallback(() => {
    setShortcutsHelpOpen((prev) => !prev);
  }, []);

  useKeyboardShortcuts({
    onRefresh: refreshPrices,
    onOpenAddHolding: handleOpenAddHolding,
    onExportCsv: handleExportCsv,
    onToggleHelp: handleToggleHelp,
  });

  // Dynamic document title
  useEffect(() => {
    if (portfolio) {
      document.title = `${formatCompact(portfolio.totalValue)} \u2014 Portfolio Tracker`;
    } else {
      document.title = 'Portfolio Tracker';
    }
  }, [portfolio]);

  // Wire errors to toast
  useEffect(() => {
    if (error) showToast(error, 'error');
  }, [error, showToast]);

  // Surface triggered price alerts as toasts
  useEffect(() => {
    if (triggeredAlertIds.length === 1) {
      showToast('Price alert triggered — check Alerts panel', 'info');
    } else if (triggeredAlertIds.length > 1) {
      showToast(`${triggeredAlertIds.length} price alerts triggered — check Alerts panel`, 'info');
    }
  }, [triggeredAlertIds, showToast]);

  // Callbacks passed to Holdings to register imperative handlers
  const handleRegisterOpenAddModal = useCallback((handler: () => void) => {
    openAddHoldingRef.current = handler;
  }, []);

  const handleRegisterExportRef = useCallback((handler: () => void) => {
    exportCsvRef.current = handler;
  }, []);

  // Show cost-basis selection modal on first launch when no method is set
  const [showCostBasisModal, setShowCostBasisModal] = useState(false);
  useEffect(() => {
    if (portfolio?.requiresCostBasisSelection) {
      setShowCostBasisModal(true);
    }
  }, [portfolio?.requiresCostBasisSelection]);

  const handleSelectCostBasis = useCallback(
    async (method: 'avco' | 'fifo') => {
      await tauriInvoke('set_config_cmd', { key: 'cost_basis_method', value: method });
      setShowCostBasisModal(false);
      await refreshPrices();
    },
    [refreshPrices]
  );

  return (
    <CurrencyContext.Provider value={{ baseCurrency, setBaseCurrency }}>
      {showCostBasisModal && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 1000,
            background: 'rgba(0,0,0,0.7)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          <div
            style={{
              background: 'var(--bg-surface)',
              border: '1px solid var(--border-primary)',
              padding: 32,
              maxWidth: 420,
              width: '100%',
            }}
          >
            <h2
              style={{
                color: 'var(--text-primary)',
                fontFamily: 'var(--font-sans)',
                margin: '0 0 12px',
                fontSize: 16,
              }}
            >
              Choose Cost-Basis Method
            </h2>
            <p
              style={{
                color: 'var(--text-secondary)',
                fontSize: 13,
                fontFamily: 'var(--font-sans)',
                margin: '0 0 24px',
              }}
            >
              Select how realized gains are calculated. This can be changed later in Settings, but
              retroactive changes will affect gain history.
            </p>
            <div style={{ display: 'flex', gap: 12 }}>
              <button
                onClick={() => handleSelectCostBasis('avco')}
                style={{
                  flex: 1,
                  padding: '10px 16px',
                  background: 'var(--color-accent)',
                  border: 'none',
                  color: '#fff',
                  cursor: 'pointer',
                  fontFamily: 'var(--font-mono)',
                  fontSize: 13,
                  borderRadius: 2,
                }}
              >
                AVCO (Average Cost)
              </button>
              <button
                onClick={() => handleSelectCostBasis('fifo')}
                style={{
                  flex: 1,
                  padding: '10px 16px',
                  background: 'var(--bg-surface-hover)',
                  border: '1px solid var(--border-primary)',
                  color: 'var(--text-primary)',
                  cursor: 'pointer',
                  fontFamily: 'var(--font-mono)',
                  fontSize: 13,
                  borderRadius: 2,
                }}
              >
                FIFO
              </button>
            </div>
          </div>
        </div>
      )}
      <Routes>
        <Route
          element={
            <Layout
              portfolio={portfolio}
              loading={loading || currencyChanging}
              isRefreshing={isRefreshing}
              isOffline={isOffline}
              onRefresh={refreshPrices}
              baseCurrency={baseCurrency}
              onBaseCurrencyChange={handleBaseCurrencyChange}
              failedSymbols={failedSymbols}
              countdown={countdown}
              unseenAlertCount={unseenTriggeredCount}
            />
          }
        >
          <Route index element={<Dashboard portfolio={portfolio} loading={loading} />} />
          <Route
            path="/holdings"
            element={
              <Holdings
                onOpenAddModal={handleRegisterOpenAddModal}
                onExportRef={handleRegisterExportRef}
              />
            }
          />
          <Route
            path="/performance"
            element={<Performance portfolio={portfolio} onRefresh={refreshPrices} />}
          />
          <Route path="/stress" element={<StressTest />} />
          <Route path="/rebalance" element={<Rebalance />} />
          <Route path="/alerts" element={<Alerts />} />
          <Route path="/transactions" element={<TransactionHistory />} />
          <Route path="/analytics" element={<Analytics />} />
          <Route path="/dividends" element={<Dividends />} />
          <Route path="/settings" element={<Settings />} />
          <Route path="/help" element={<Help />} />
        </Route>
      </Routes>
      <KeyboardShortcutsOverlay
        isOpen={shortcutsHelpOpen}
        onClose={() => setShortcutsHelpOpen(false)}
      />
    </CurrencyContext.Provider>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <PortfolioProvider>
        <BrowserRouter>
          <AppRoutes />
        </BrowserRouter>
      </PortfolioProvider>
    </ToastProvider>
  );
}
