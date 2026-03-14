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

  // Dynamic document title
  useEffect(() => {
    if (portfolio) {
      document.title = `${formatCompact(portfolio.totalValue)} — Portfolio Tracker`;
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
        refreshPrices();
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
    <Routes>
      <Route element={<Layout portfolio={portfolio} loading={loading} onRefresh={refreshPrices} />}>
        <Route index element={<Dashboard portfolio={portfolio} loading={loading} />} />
        <Route path="/holdings" element={<Holdings />} />
        <Route path="/performance" element={<Performance portfolio={portfolio} />} />
        <Route path="/stress" element={<StressTest />} />
      </Route>
    </Routes>
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
