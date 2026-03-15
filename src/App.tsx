import { useCallback, useEffect, useRef, useState } from 'react';
import { BrowserRouter, Routes, Route, useNavigate } from 'react-router-dom';
import { Layout } from './components/Layout';
import { Dashboard } from './components/Dashboard';
import { Holdings } from './components/Holdings';
import { Performance } from './components/Performance';
import { StressTest } from './components/StressTest';
import { ToastProvider } from './components/ui/Toast';
import { useToast } from './components/ui/Toast';
import { KeyboardShortcutsOverlay } from './components/ui/KeyboardShortcutsOverlay';
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts';
import { PortfolioProvider, usePortfolio } from './hooks/usePortfolio';
import { useConfig } from './hooks/useConfig';
import { CurrencyContext } from './lib/currencyContext';
import { formatCompact } from './lib/format';

function AppRoutes() {
  const { portfolio, loading, error, refreshPrices } = usePortfolio();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { value: baseCurrency, setValue: setBaseCurrency } = useConfig('base_currency', 'CAD');

  const [shortcutsHelpOpen, setShortcutsHelpOpen] = useState(false);

  // Refs for imperative handles registered by Holdings
  const openAddHoldingRef = useRef<(() => void) | null>(null);
  const exportCsvRef = useRef<(() => void) | null>(null);

  const handleOpenAddHolding = useCallback(() => {
    // Navigate to holdings first, then open modal
    navigate('/holdings');
    // The Holdings component registers its handler via onOpenAddModal;
    // use a short defer so the route renders before firing the callback.
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
              onBaseCurrencyChange={setBaseCurrency}
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
          <Route path="/performance" element={<Performance portfolio={portfolio} />} />
          <Route path="/stress" element={<StressTest />} />
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
