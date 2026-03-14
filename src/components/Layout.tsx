import { Outlet } from 'react-router-dom';
import { Sidebar } from './Sidebar';
import { TopBar } from './TopBar';
import type { PortfolioSnapshot } from '../types/portfolio';

interface LayoutProps {
  portfolio: PortfolioSnapshot | null;
  loading: boolean;
  onRefresh: () => void;
  baseCurrency: string;
  onBaseCurrencyChange: (currency: string) => void;
}

export function Layout({
  portfolio,
  loading,
  onRefresh,
  baseCurrency,
  onBaseCurrencyChange,
}: LayoutProps) {
  return (
    <div
      style={{
        display: 'flex',
        height: '100vh',
        overflow: 'hidden',
        background: 'var(--bg-primary)',
      }}
    >
      <Sidebar portfolio={portfolio} />
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <TopBar
          portfolio={portfolio}
          loading={loading}
          onRefresh={onRefresh}
          baseCurrency={baseCurrency}
          onBaseCurrencyChange={onBaseCurrencyChange}
        />
        <main
          style={{
            flex: 1,
            minHeight: 0,
            overflowY: 'auto',
            padding: '24px',
            scrollbarWidth: 'thin',
            scrollbarColor: 'var(--border-primary) transparent',
          }}
        >
          <Outlet />
        </main>
      </div>
    </div>
  );
}
