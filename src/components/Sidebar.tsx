import { useState } from 'react';
import { NavLink } from 'react-router-dom';
import { LayoutDashboard, Table2, TrendingUp, AlertTriangle } from 'lucide-react';
import { formatCompact } from '../lib/format';
import { pnlColor } from '../lib/colors';
import type { PortfolioSnapshot } from '../types/portfolio';

interface SidebarProps {
  portfolio: PortfolioSnapshot | null;
}

const NAV_ITEMS = [
  { to: '/', label: 'Dashboard', Icon: LayoutDashboard },
  { to: '/holdings', label: 'Holdings', Icon: Table2 },
  { to: '/performance', label: 'Performance', Icon: TrendingUp },
  { to: '/stress', label: 'Stress Test', Icon: AlertTriangle },
];

export function Sidebar({ portfolio }: SidebarProps) {
  const [expanded, setExpanded] = useState(false);
  const totalValue = portfolio?.totalValue ?? 0;
  const dailyPnl = portfolio?.dailyPnl ?? 0;

  return (
    <nav
      onMouseEnter={() => setExpanded(true)}
      onMouseLeave={() => setExpanded(false)}
      style={{
        width: expanded ? 220 : 64,
        minWidth: expanded ? 220 : 64,
        transition: 'width 200ms ease, min-width 200ms ease',
        background: 'var(--bg-surface)',
        borderRight: '1px solid var(--border-primary)',
        display: 'flex',
        flexDirection: 'column',
        height: '100vh',
        position: 'sticky',
        top: 0,
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
        {NAV_ITEMS.map(({ to, label, Icon }) => (
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
            color: pnlColor(totalValue),
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
    </nav>
  );
}
