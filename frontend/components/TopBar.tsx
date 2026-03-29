import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw, ChevronDown, AlertTriangle } from 'lucide-react';
import { useLocation } from 'react-router-dom';
import { formatCurrency, formatPercent } from '../lib/format';
import { pnlColor } from '../lib/colors';
import { SUPPORTED_CURRENCIES } from '../lib/constants';
import type { PortfolioSnapshot } from '../types/portfolio';

interface TopBarProps {
  portfolio: PortfolioSnapshot | null;
  loading: boolean;
  isRefreshing?: boolean | undefined;
  isOffline?: boolean | undefined;
  onRefresh: () => void;
  baseCurrency: string;
  onBaseCurrencyChange: (currency: string) => void;
  failedSymbols?: string[] | undefined;
  countdown?: number | null | undefined;
}

const ROUTE_TITLE_KEYS: Record<string, string> = {
  '/': 'nav.dashboard',
  '/holdings': 'nav.holdings',
  '/performance': 'nav.performance',
  '/stress': 'nav.stressTest',
  '/rebalance': 'nav.rebalance',
  '/alerts': 'nav.alerts',
  '/transactions': 'nav.transactions',
  '/analytics': 'nav.analytics',
  '/dividends': 'nav.dividends',
  '/help': 'Keyboard Shortcuts',
  '/settings': 'nav.settings',
};

function useRelativeTime(isoDate: string | null): string {
  const [label, setLabel] = useState('\u2014');

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

interface CurrencyPickerProps {
  value: string;
  onChange: (currency: string) => void;
}

function CurrencyPicker({ value, onChange }: CurrencyPickerProps) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, []);

  return (
    <div ref={ref} style={{ position: 'relative' }}>
      <button
        onClick={() => setOpen((prev) => !prev)}
        title="Change base currency"
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 4,
          padding: '5px 10px',
          background: 'transparent',
          border: '1px solid var(--border-primary)',
          color: 'var(--text-secondary)',
          borderRadius: '2px',
          cursor: 'pointer',
          fontSize: 12,
          fontFamily: 'var(--font-mono)',
          letterSpacing: '0.04em',
        }}
      >
        {value}
        <ChevronDown size={11} />
      </button>

      {open && (
        <div
          style={{
            position: 'absolute',
            top: 'calc(100% + 4px)',
            right: 0,
            background: 'var(--bg-surface)',
            border: '1px solid var(--border-primary)',
            borderRadius: '2px',
            zIndex: 20,
            minWidth: 90,
            overflow: 'hidden',
          }}
        >
          {SUPPORTED_CURRENCIES.map((curr) => (
            <button
              key={curr}
              onClick={() => {
                onChange(curr);
                setOpen(false);
              }}
              style={{
                display: 'block',
                width: '100%',
                textAlign: 'left',
                padding: '7px 12px',
                background: curr === value ? 'var(--bg-surface-hover)' : 'transparent',
                color: curr === value ? 'var(--text-primary)' : 'var(--text-secondary)',
                border: 'none',
                cursor: 'pointer',
                fontSize: 12,
                fontFamily: 'var(--font-mono)',
                letterSpacing: '0.04em',
              }}
            >
              {curr}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function formatCountdown(seconds: number): string {
  if (seconds <= 0) return '';
  if (seconds >= 3600) return `${Math.floor(seconds / 3600)}h`;
  if (seconds >= 60) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  return `${seconds}s`;
}

export function TopBar({
  portfolio,
  loading,
  isRefreshing = false,
  isOffline = false,
  onRefresh,
  baseCurrency,
  onBaseCurrencyChange,
  failedSymbols = [],
  countdown = null,
}: TopBarProps) {
  const { t } = useTranslation();
  const { pathname } = useLocation();
  const titleKey = ROUTE_TITLE_KEYS[pathname];
  const title = titleKey
    ? titleKey.startsWith('nav.')
      ? t(titleKey)
      : titleKey
    : 'Portfolio Tracker';
  const updatedLabel = useRelativeTime(portfolio?.lastUpdated ?? null);
  const dailyPnl = portfolio?.dailyPnl ?? 0;
  const prevValue = portfolio ? portfolio.totalValue - dailyPnl : 0;
  const rawDailyPct = prevValue !== 0 ? (dailyPnl / prevValue) * 100 : 0;
  const dailyPct = Number.isFinite(rawDailyPct) ? rawDailyPct : 0;

  const isBusy = loading || isRefreshing;
  // Flash the countdown label in the last 10 seconds before refresh
  const isUrgent = !isBusy && countdown !== null && countdown < 10;

  return (
    <div style={{ position: 'sticky', top: 0, zIndex: 5 }}>
      <div
        style={{
          background: 'var(--bg-primary)',
          borderBottom:
            failedSymbols.length > 0 || isOffline ? 'none' : '1px solid var(--border-primary)',
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
              {dailyPnl >= 0 ? '+' : ''}
              {formatCurrency(dailyPnl, baseCurrency)} ({formatPercent(dailyPct)})
            </span>
          )}

          {/* Last updated / countdown */}
          <span
            style={{
              fontSize: 11,
              color: isRefreshing ? 'var(--color-accent)' : 'var(--text-muted)',
              fontFamily: 'var(--font-mono)',
              animation: isUrgent ? 'pulse 1s ease-in-out infinite' : 'none',
            }}
          >
            {loading
              ? 'Loading...'
              : isRefreshing
                ? 'Refreshing...'
                : countdown !== null
                  ? `Auto-refreshing in ${formatCountdown(countdown)}`
                  : `Updated ${updatedLabel}`}
          </span>

          {/* Currency picker */}
          <CurrencyPicker value={baseCurrency} onChange={onBaseCurrencyChange} />

          {/* Refresh button */}
          <button
            onClick={onRefresh}
            disabled={isBusy}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '5px 12px',
              background: 'transparent',
              border: '1px solid var(--border-primary)',
              color: isBusy ? 'var(--text-muted)' : 'var(--text-secondary)',
              borderRadius: '2px',
              cursor: isBusy ? 'not-allowed' : 'pointer',
              fontSize: 12,
              fontFamily: 'var(--font-sans)',
            }}
          >
            <RefreshCw
              size={13}
              style={{ animation: isBusy ? 'spin 0.7s linear infinite' : 'none' }}
            />
            {t('common.refresh')}
          </button>
        </div>
      </div>

      {/* Offline banner */}
      {isOffline && (
        <div
          style={{
            background: 'rgba(59,130,246,0.08)',
            borderBottom: failedSymbols.length > 0 ? 'none' : '1px solid var(--border-primary)',
            borderTop: '1px solid rgba(59,130,246,0.3)',
            padding: '6px 24px',
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            fontSize: 11,
            fontFamily: 'var(--font-mono)',
            color: 'var(--color-accent)',
          }}
        >
          <AlertTriangle size={12} />
          <span>
            Offline — showing last-known portfolio
            {portfolio?.lastUpdated ? ` (${new Date(portfolio.lastUpdated).toLocaleString()})` : ''}
          </span>
          <button
            onClick={onRefresh}
            disabled={isBusy}
            style={{
              marginLeft: 'auto',
              background: 'none',
              border: '1px solid var(--color-accent)',
              color: 'var(--color-accent)',
              fontSize: 10,
              fontFamily: 'var(--font-mono)',
              padding: '2px 8px',
              cursor: 'pointer',
              borderRadius: '2px',
              opacity: isBusy ? 0.5 : 1,
            }}
          >
            Retry
          </button>
        </div>
      )}

      {/* Failed symbols warning banner */}
      {failedSymbols.length > 0 && (
        <div
          style={{
            background: 'rgba(251,191,36,0.08)',
            borderBottom: '1px solid var(--border-primary)',
            borderTop: '1px solid rgba(251,191,36,0.3)',
            padding: '6px 24px',
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            fontSize: 11,
            fontFamily: 'var(--font-mono)',
            color: 'var(--color-warning)',
          }}
        >
          <AlertTriangle size={12} />
          <span>Price refresh failed for: {failedSymbols.join(', ')} — showing cached prices</span>
          <button
            onClick={onRefresh}
            disabled={isBusy}
            style={{
              marginLeft: 'auto',
              background: 'none',
              border: '1px solid var(--color-warning)',
              color: 'var(--color-warning)',
              fontSize: 10,
              fontFamily: 'var(--font-mono)',
              padding: '2px 8px',
              cursor: 'pointer',
              borderRadius: '2px',
              opacity: isBusy ? 0.5 : 1,
            }}
          >
            Retry
          </button>
        </div>
      )}
    </div>
  );
}
