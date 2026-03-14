import { useEffect } from 'react';
import { BrowserRouter, Routes, Route, useNavigate } from 'react-router-dom';
import { Layout } from './components/Layout';
import { Dashboard } from './components/Dashboard';
import { Holdings } from './components/Holdings';
import { Performance } from './components/Performance';
import { StressTest } from './components/StressTest';
import { ToastProvider } from './components/ui/Toast';
import { useToast } from './components/ui/Toast';
import { usePortfolio } from './hooks/usePortfolio';
import { useConfig } from './hooks/useConfig';
import { CurrencyContext } from './lib/currencyContext';
import { formatCompact } from './lib/format';

const ROUTE_KEYS: Record<string, string> = {
  '1': '/',
  '2': '/holdings',
  '3': '/performance',
  '4': '/stress',
};

function AppRoutes() {
  const { portfolio, loading, error, refreshPrices } = usePortfolio();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { value: baseCurrency, setValue: setBaseCurrency } = useConfig('base_currency', 'CAD');

  // Dynamic document title
  useEffect(() => {
    if (portfolio) {
      document.title = `${formatCompact(portfolio.totalValue)} \u2014 Portfolio Tracker`;
    } else {
      document.title = 'Portfolio Tracker';
    }
  }, [portfolio]);

  // Keyboard shortcuts
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      const isMeta = e.metaKey || e.ctrlKey;
      const tag = (e.target as HTMLElement).tagName;
      const isInput = tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA';

      // Cmd/Ctrl+R \u2014 refresh prices
      if (isMeta && e.key === 'r' && !isInput) {
        e.preventDefault();
        refreshPrices();
        return;
      }

      // 1\u20134 \u2014 navigate views (when not in an input)
      if (!isMeta && !isInput && ROUTE_KEYS[e.key]) {
        navigate(ROUTE_KEYS[e.key]);
      }
    }

    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [navigate, refreshPrices]);

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
    </CurrencyContext.Provider>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <BrowserRouter>
        <AppRoutes />
      </BrowserRouter>
    </ToastProvider>
  );
}
