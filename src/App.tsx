import { useCallback, useEffect, useRef, useState } from 'react';
import { BrowserRouter, Routes, Route, useNavigate } from 'react-router-dom';
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
import { ToastProvider } from './components/ui/Toast';
import { useToast } from './components/ui/Toast';
import { KeyboardShortcutsOverlay } from './components/ui/KeyboardShortcutsOverlay';
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts';
import { PortfolioProvider, usePortfolio } from './hooks/usePortfolio';
import { useConfig } from './hooks/useConfig';
import { useAutoRefresh } from './hooks/useAutoRefresh';
import { CurrencyContext } from './lib/currencyContext';
import { formatCompact } from './lib/format';

function AppRoutes() {
  const { portfolio, loading, error, failedSymbols, refreshPrices } = usePortfolio();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { value: baseCurrency, setValue: setBaseCurrency } = useConfig('base_currency', 'CAD');
  const [shortcutsHelpOpen, setShortcutsHelpOpen] = useState(false);

  const { countdown } = useAutoRefresh({
    onRefresh: refreshPrices,
  });

  // Refs for imperative handles registered by Holdings
  const openAddHoldingRef = useRef<(() => void) | null>(null);
  const exportCsvRef = useRef<(() => void) | null>(null);

  // When base currency changes, re-fetch prices so conversions update immediately (#98)
  const handleBaseCurrencyChange = useCallback(
    (currency: string) => {
      setBaseCurrency(currency);
      void refreshPrices();
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

  // Callbacks passed to Holdings to register imperative handlers
  const handleRegisterOpenAddModal = useCallback((handler: () => void) => {
    openAddHoldingRef.current = handler;
  }, []);

  const handleRegisterExportRef = useCallback((handler: () => void) => {
    exportCsvRef.current = handler;
  }, []);

  return (
    <CurrencyContext.Provider value={{ baseCurrency, setBaseCurrency }}>
      <Routes>
        <Route
          element={
            <Layout
              portfolio={portfolio}
              loading={loading}
              onRefresh={refreshPrices}
              baseCurrency={baseCurrency}
              onBaseCurrencyChange={handleBaseCurrencyChange}
              failedSymbols={failedSymbols}
              countdown={countdown}
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
