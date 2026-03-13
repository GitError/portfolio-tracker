import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { Layout } from './components/Layout';
import { Dashboard } from './components/Dashboard';
import { Holdings } from './components/Holdings';
import { Performance } from './components/Performance';
import { StressTest } from './components/StressTest';
import { ToastProvider } from './components/ui/Toast';
import { usePortfolio } from './hooks/usePortfolio';

function AppRoutes() {
  const { portfolio, loading, refreshPrices } = usePortfolio();

  return (
    <Routes>
      <Route element={<Layout portfolio={portfolio} loading={loading} onRefresh={refreshPrices} />}>
        <Route index element={<Dashboard portfolio={portfolio} loading={loading} />} />
        <Route path="/holdings" element={<Holdings />} />
        <Route path="/performance" element={<Performance />} />
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
