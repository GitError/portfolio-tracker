import { useCallback, useEffect } from 'react';
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
import { useAutoRefresh } from './hooks/useAutoRefresh';
import { CurrencyContext } from './lib/currencyContext';
import { formatCompact } from './lib/format';

// Auto-refresh interval options in milliseconds (0 = disabled)
const AUTO_REFRESH_INTERVALS: Record<string, number> = {
  '0': 0,
  '60000': 60_000,
  '300000': 300_000,
  '900000': 900_000,
  '1800000': 1_800_000,
  '3600000': 3_600_000,
};

const ROUTE_KEYS: Record<string, string> = {
  '1': '/',
  '2': '/holdings',
  '3': '/performance',
  '4': '/stress',
};

function AppRoutes() {
  const { portfolio, loading, error, failedSymbols, refreshPrices } = usePortfolio();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { value: baseCurrency, setValue: setBaseCurrency } = useConfig('base_currency', 'CAD');
  const { value: autoRefreshStr } = useConfig('auto_refresh_interval_ms', '0');

  const autoRefreshMs = AUTO_REFRESH_INTERVALS[autoRefreshStr] ?? 0;

  const { countdown } = useAutoRefresh({
    intervalMs: autoRefreshMs,
    onRefresh: refreshPrices,
  });

  // When base currency changes, re-fetch prices so conversions update immediately (#98)
  const handleBaseCurrencyChange = useCallback(
    (currency: string) => {
      setBaseCurrency(currency);
      void refreshPrices();
    },
    [setBaseCurrency, refreshPrices]
  );

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

      // Cmd/Ctrl+R — refresh prices
      if (isMeta && e.key === 'r' && !isInput) {
        e.preventDefault();
        void refreshPrices();
        return;
      }

      // 1–4 — navigate views (when not in an input)
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
              onBaseCurrencyChange={handleBaseCurrencyChange}
              failedSymbols={failedSymbols}
              countdown={countdown}
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
      <PortfolioProvider>
        <BrowserRouter>
          <AppRoutes />
        </BrowserRouter>
      </PortfolioProvider>
    </ToastProvider>
  );
}
