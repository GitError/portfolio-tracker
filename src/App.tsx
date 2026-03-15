import { useEffect, useState, useCallback } from 'react';
import { BrowserRouter, Routes, Route, useNavigate } from 'react-router-dom';
import { Layout } from './components/Layout';
import { Dashboard } from './components/Dashboard';
import { Holdings } from './components/Holdings';
import { Performance } from './components/Performance';
import { StressTest } from './components/StressTest';
import { ToastProvider } from './components/ui/Toast';
import { useToast } from './components/ui/Toast';
import { PortfolioProvider, usePortfolio } from './hooks/usePortfolio';
import { useConfig } from './hooks/useConfig';
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts';
import { KeyboardShortcutsOverlay } from './components/KeyboardShortcutsOverlay';
import { CurrencyContext } from './lib/currencyContext';
import { formatCompact } from './lib/format';

function AppRoutes() {
  const { portfolio, loading, error, refreshPrices } = usePortfolio();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { value: baseCurrency, setValue: setBaseCurrency } = useConfig('base_currency', 'CAD');
  const [showShortcutsHelp, setShowShortcutsHelp] = useState(false);

  // Dynamic document title
  useEffect(() => {
    if (portfolio) {
      document.title = `${formatCompact(portfolio.totalValue)} \u2014 Portfolio Tracker`;
    } else {
      document.title = 'Portfolio Tracker';
    }
  }, [portfolio]);

  const handleAddHolding = useCallback(() => {
    navigate('/holdings?add=1');
  }, [navigate]);

  const handleToggleHelp = useCallback(() => {
    setShowShortcutsHelp((prev) => !prev);
  }, []);

  // Keyboard shortcuts via dedicated hook
  useKeyboardShortcuts({
    onRefresh: refreshPrices,
    onAddHolding: handleAddHolding,
    onNavigate: navigate,
    onToggleHelp: handleToggleHelp,
  });

  // Wire errors to toast
  useEffect(() => {
    if (error) showToast(error, 'error');
  }, [error, showToast]);

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
          <Route path="/holdings" element={<Holdings />} />
          <Route path="/performance" element={<Performance portfolio={portfolio} />} />
          <Route path="/stress" element={<StressTest />} />
        </Route>
      </Routes>
      <KeyboardShortcutsOverlay
        isOpen={showShortcutsHelp}
        onClose={() => setShowShortcutsHelp(false)}
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
