import { useState, useEffect } from 'react';
import { NavLink } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  LayoutDashboard,
  Table2,
  TrendingUp,
  AlertTriangle,
  Scale,
  Bell,
  Receipt,
  BarChart2,
  DollarSign,
  Settings2,
  HelpCircle,
  ChevronsLeft,
  ChevronsRight,
} from 'lucide-react';
import { formatCompact } from '../lib/format';
import { pnlColor } from '../lib/colors';
import type { PortfolioSnapshot } from '../types/portfolio';

const STORAGE_KEY = 'sidebar-expanded';

interface SidebarProps {
  portfolio: PortfolioSnapshot | null;
}

const NAV_ITEM_DEFS = [
  { to: '/', key: 'nav.dashboard', Icon: LayoutDashboard },
  { to: '/holdings', key: 'nav.holdings', Icon: Table2 },
  { to: '/performance', key: 'nav.performance', Icon: TrendingUp },
  { to: '/stress', key: 'nav.stressTest', Icon: AlertTriangle },
  { to: '/rebalance', key: 'nav.rebalance', Icon: Scale },
  { to: '/alerts', key: 'nav.alerts', Icon: Bell },
  { to: '/transactions', key: 'nav.transactions', Icon: Receipt },
  { to: '/analytics', key: 'nav.analytics', Icon: BarChart2 },
  { to: '/dividends', key: 'nav.dividends', Icon: DollarSign },
  { to: '/settings', key: 'nav.settings', Icon: Settings2 },
  { to: '/help', key: 'nav.help', Icon: HelpCircle },
];

export function Sidebar({ portfolio }: SidebarProps) {
  const { t } = useTranslation();
  const navItems = NAV_ITEM_DEFS.map((item) => ({ ...item, label: t(item.key) }));
  const [expanded, setExpanded] = useState(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored === null ? false : stored === 'true';
  });
  const totalValue = portfolio?.totalValue ?? 0;
  const dailyPnl = portfolio?.dailyPnl ?? 0;

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, String(expanded));
  }, [expanded]);

  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.metaKey && e.key === 'b') {
        e.preventDefault();
        setExpanded((prev) => !prev);
      }
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, []);

  return (
    <nav
      style={{
        width: expanded ? 220 : 64,
        minWidth: expanded ? 220 : 64,
        transition: 'width 200ms ease, min-width 200ms ease',
        background: 'var(--bg-surface)',
        borderRight: '1px solid var(--border-primary)',
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        overflow: 'hidden',
        flexShrink: 0,
        zIndex: 10,
      }}
    >
      {/* Logo */}
      <div
        style={{
          padding: '20px 0',
          display: 'flex',
          alignItems: 'center',
          justifyContent: expanded ? 'flex-start' : 'center',
          paddingLeft: expanded ? 20 : 0,
          gap: 10,
          borderBottom: '1px solid var(--border-subtle)',
          minHeight: 60,
        }}
      >
        <span
          style={{
            fontFamily: 'var(--font-mono)',
            fontWeight: 700,
            fontSize: 14,
            color: 'var(--color-accent)',
            whiteSpace: 'nowrap',
          }}
        >
          {expanded ? 'Portfolio Tracker' : 'PT'}
        </span>
      </div>

      {/* Nav */}
      <div style={{ flex: 1, paddingTop: 8 }}>
        {navItems.map(({ to, label, Icon }) => (
          <NavLink
            key={to}
            to={to}
            end={to === '/'}
            style={({ isActive }) => ({
              display: 'flex',
              alignItems: 'center',
              gap: 12,
              padding: '10px 0',
              paddingLeft: expanded ? 20 : 0,
              justifyContent: expanded ? 'flex-start' : 'center',
              textDecoration: 'none',
              color: isActive ? 'var(--text-primary)' : 'var(--text-secondary)',
              background: isActive ? 'var(--bg-surface-hover)' : 'transparent',
              borderLeft: isActive ? '2px solid var(--color-accent)' : '2px solid transparent',
              transition: 'color 150ms, background 150ms',
              fontSize: 13,
              fontWeight: isActive ? 600 : 400,
              whiteSpace: 'nowrap',
            })}
          >
            <Icon
              size={18}
              style={{
                flexShrink: 0,
                marginLeft: expanded ? 0 : 'auto',
                marginRight: expanded ? 0 : 'auto',
              }}
            />
            {expanded && label}
          </NavLink>
        ))}
      </div>

      {/* Portfolio mini value */}
      <div
        style={{
          padding: expanded ? '16px 20px' : '16px 0',
          borderTop: '1px solid var(--border-subtle)',
          textAlign: expanded ? 'left' : 'center',
        }}
      >
        <div
          style={{
            fontFamily: 'var(--font-mono)',
            fontSize: 13,
            fontWeight: 600,
            color: 'var(--text-primary)',
            whiteSpace: 'nowrap',
          }}
        >
          {formatCompact(totalValue)}
        </div>
        {expanded && (
          <div
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 11,
              color: pnlColor(dailyPnl),
              marginTop: 2,
            }}
          >
            {dailyPnl >= 0 ? '+' : ''}
            {formatCompact(Math.abs(dailyPnl))} today
          </div>
        )}
      </div>

      {/* Toggle button */}
      <button
        onClick={() => setExpanded((prev) => !prev)}
        title={expanded ? 'Collapse sidebar (⌘B)' : 'Expand sidebar (⌘B)'}
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          width: '100%',
          padding: '12px 0',
          background: 'transparent',
          border: 'none',
          borderTop: '1px solid var(--border-subtle)',
          cursor: 'pointer',
          color: 'var(--text-secondary)',
          transition: 'color 150ms',
        }}
        onMouseEnter={(e) => {
          (e.currentTarget as HTMLButtonElement).style.color = 'var(--text-primary)';
        }}
        onMouseLeave={(e) => {
          (e.currentTarget as HTMLButtonElement).style.color = 'var(--text-secondary)';
        }}
      >
        {expanded ? <ChevronsLeft size={16} /> : <ChevronsRight size={16} />}
      </button>
    </nav>
  );
}
