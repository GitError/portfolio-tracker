import { useEffect, useState } from 'react';
import { RefreshCw } from 'lucide-react';
import { useLocation } from 'react-router-dom';
import { formatCurrency, formatPercent } from '../lib/format';
import { pnlColor } from '../lib/colors';
import type { PortfolioSnapshot } from '../types/portfolio';

interface TopBarProps {
  portfolio: PortfolioSnapshot | null;
  loading: boolean;
  onRefresh: () => void;
}

const ROUTE_TITLES: Record<string, string> = {
  '/':            'Dashboard',
  '/holdings':    'Holdings',
  '/performance': 'Performance',
  '/stress':      'Stress Test',
};

function useRelativeTime(isoDate: string | null): string {
  const [label, setLabel] = useState('—');

  useEffect(() => {
    if (!isoDate) return;
    function update() {
      const diff = Math.floor((Date.now() - new Date(isoDate!).getTime()) / 1000);
      if (diff < 60) setLabel(`${diff}s ago`);
      else if (diff < 3600) setLabel(`${Math.floor(diff / 60)}m ago`);
      else setLabel(`${Math.floor(diff / 3600)}h ago`);
    }
    update();
    const id = setInterval(update, 15_000);
    return () => clearInterval(id);
  }, [isoDate]);

  return label;
}

export function TopBar({ portfolio, loading, onRefresh }: TopBarProps) {
  const { pathname } = useLocation();
  const title = ROUTE_TITLES[pathname] ?? 'Portfolio Tracker';
  const updatedLabel = useRelativeTime(portfolio?.lastUpdated ?? null);
  const dailyPnl = portfolio?.dailyPnl ?? 0;
  const dailyPct = portfolio
    ? (dailyPnl / (portfolio.totalValue - dailyPnl)) * 100
    : 0;

  return (
    <div
      style={{
        position: 'sticky',
        top: 0,
        zIndex: 5,
        background: 'var(--bg-primary)',
        borderBottom: '1px solid var(--border-primary)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '0 24px',
        height: 56,
        flexShrink: 0,
      }}
    >
      <h1
        style={{
          fontFamily: 'var(--font-sans)',
          fontWeight: 600,
          fontSize: 16,
          color: 'var(--text-primary)',
          letterSpacing: '-0.01em',
        }}
      >
        {title}
      </h1>

      <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
        {/* Daily P&L badge */}
        {portfolio && (
          <span
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 12,
              color: pnlColor(dailyPnl),
              background: 'var(--bg-surface)',
              border: `1px solid ${pnlColor(dailyPnl)}22`,
              padding: '3px 8px',
              borderRadius: '2px',
            }}
          >
            {dailyPnl >= 0 ? '+' : ''}{formatCurrency(dailyPnl)} ({formatPercent(dailyPct)})
          </span>
        )}

        {/* Last updated */}
        <span
          style={{
            fontSize: 11,
            color: 'var(--text-muted)',
            fontFamily: 'var(--font-mono)',
          }}
        >
          {loading ? 'Refreshing...' : `Updated ${updatedLabel}`}
        </span>

        {/* Refresh button */}
        <button
          onClick={onRefresh}
          disabled={loading}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            padding: '5px 12px',
            background: 'transparent',
            border: '1px solid var(--border-primary)',
            color: loading ? 'var(--text-muted)' : 'var(--text-secondary)',
            borderRadius: '2px',
            cursor: loading ? 'not-allowed' : 'pointer',
            fontSize: 12,
            fontFamily: 'var(--font-sans)',
          }}
        >
          <RefreshCw
            size={13}
            style={{ animation: loading ? 'spin 0.7s linear infinite' : 'none' }}
          />
          Refresh
        </button>
      </div>
    </div>
  );
}
