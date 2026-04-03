import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { PieChart, Pie, Cell, Tooltip, ResponsiveContainer } from 'recharts';
import { formatCurrency, formatCompact, formatNumber, formatPercent } from '../lib/format';
import { pnlColor } from '../lib/colors';
import {
  ACCOUNT_OPTIONS,
  ACCOUNT_TYPE_CONFIG,
  ASSET_TYPE_CONFIG,
  CURRENCY_COLORS,
} from '../lib/constants';
import { EmptyState } from './ui/EmptyState';
import { ActionCenter } from './ActionCenter';
import { useActionInsights } from '../hooks/useActionInsights';
import { config } from '../lib/config';
import { Select } from './ui/Select';
import type { AccountType, HoldingWithPrice, PortfolioSnapshot } from '../types/portfolio';

interface DashboardProps {
  portfolio: PortfolioSnapshot | null;
  loading: boolean;
  /** Called when the user clicks "Set method" in the realized-gains stat (#488). */
  onOpenCostBasisModal?: () => void;
}

const PANEL: React.CSSProperties = {
  background: 'var(--bg-surface)',
  border: '1px solid var(--border-primary)',
  padding: '20px',
  borderRadius: 0,
};

const LABEL: React.CSSProperties = {
  fontSize: 11,
  color: 'var(--text-muted)',
  fontFamily: 'var(--font-mono)',
  textTransform: 'uppercase',
  letterSpacing: '0.08em',
  marginBottom: 8,
};

function CenterLabel({ text }: { text: string }) {
  return (
    <text
      x="50%"
      y="50%"
      textAnchor="middle"
      dominantBaseline="central"
      fill="var(--text-secondary)"
      fontSize={11}
      fontFamily="var(--font-mono)"
      style={{ textTransform: 'uppercase', letterSpacing: '0.08em' }}
    >
      {text}
    </text>
  );
}

function topMoversTitle(lastUpdated: string | undefined): string {
  if (!lastUpdated) return 'Top Movers';
  const updated = new Date(lastUpdated);
  const now = new Date();
  const isToday =
    updated.getFullYear() === now.getFullYear() &&
    updated.getMonth() === now.getMonth() &&
    updated.getDate() === now.getDate();
  return isToday ? 'Top Movers \u2014 Today' : 'Top Movers \u2014 Last Close';
}

export function Dashboard({ portfolio, loading, onOpenCostBasisModal }: DashboardProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const accountFilter = (searchParams.get('account') ?? 'all') as 'all' | AccountType;

  function setAccountFilter(value: 'all' | AccountType) {
    setSearchParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        if (value === 'all') {
          next.delete('account');
        } else {
          next.set('account', value);
        }
        return next;
      },
      { replace: true }
    );
  }

  const baseCurrency = portfolio?.baseCurrency ?? 'CAD';
  const filteredHoldings = useMemo(() => {
    if (!portfolio) return [];
    return accountFilter === 'all'
      ? portfolio.holdings
      : portfolio.holdings.filter((holding) => holding.account === accountFilter);
  }, [portfolio, accountFilter]);

  const totals = useMemo(() => {
    const totalValue = filteredHoldings.reduce((sum, holding) => sum + holding.marketValueCad, 0);
    const totalCost = filteredHoldings.reduce((sum, holding) => sum + holding.costValueCad, 0);
    const totalGainLoss = totalValue - totalCost;
    return {
      totalValue,
      totalCost,
      totalGainLoss,
      totalGainLossPercent: totalCost > 0 ? (totalGainLoss / totalCost) * 100 : 0,
      dailyPnl: filteredHoldings.reduce(
        (sum, holding) => sum + holding.marketValueCad * (holding.dailyChangePercent / 100),
        0
      ),
    };
  }, [filteredHoldings]);

  const [allocView, setAllocView] = useState<'type' | 'account'>('type');
  const actionInsights = useActionInsights(portfolio, filteredHoldings as HoldingWithPrice[]);

  const allocationData = useMemo(() => {
    if (!portfolio || totals.totalValue === 0) return [];
    const byType: Record<string, number> = {};
    for (const h of filteredHoldings) {
      byType[h.assetType] = (byType[h.assetType] ?? 0) + h.marketValueCad;
    }
    return Object.entries(byType).map(([type, value]) => ({
      name: ASSET_TYPE_CONFIG[type as keyof typeof ASSET_TYPE_CONFIG]?.label ?? type,
      value: Math.round(value * 100) / 100,
      color: ASSET_TYPE_CONFIG[type as keyof typeof ASSET_TYPE_CONFIG]?.color ?? '#888',
      pct: (value / totals.totalValue) * 100,
    }));
  }, [filteredHoldings, portfolio, totals]);

  const accountAllocationData = useMemo(() => {
    if (!portfolio || totals.totalValue === 0) return [];
    const byAccount: Record<string, number> = {};
    for (const h of filteredHoldings) {
      const key = h.account ?? 'other';
      byAccount[key] = (byAccount[key] ?? 0) + h.marketValueCad;
    }
    return Object.entries(byAccount).map(([account, value]) => ({
      name: ACCOUNT_TYPE_CONFIG[account]?.label ?? account,
      value: Math.round(value * 100) / 100,
      color: ACCOUNT_TYPE_CONFIG[account]?.color ?? 'var(--text-muted)',
      pct: (value / totals.totalValue) * 100,
    }));
  }, [filteredHoldings, portfolio, totals]);

  const currencyData = useMemo(() => {
    if (!portfolio || totals.totalValue === 0) return [];
    const byCcy: Record<string, number> = {};
    for (const h of filteredHoldings) {
      byCcy[h.currency] = (byCcy[h.currency] ?? 0) + h.marketValueCad;
    }
    return Object.entries(byCcy).map(([ccy, value]) => ({
      name: ccy,
      value: Math.round(value * 100) / 100,
      color: CURRENCY_COLORS[ccy] ?? '#888',
      pct: (value / totals.totalValue) * 100,
    }));
  }, [filteredHoldings, portfolio, totals]);

  const nonCashHoldings = useMemo(
    () => filteredHoldings.filter((h) => h.assetType !== 'cash'),
    [filteredHoldings]
  );

  const topGainers = useMemo(
    () =>
      [...nonCashHoldings].sort((a, b) => b.dailyChangePercent - a.dailyChangePercent).slice(0, 3),
    [nonCashHoldings]
  );

  const topLosers = useMemo(
    () =>
      [...nonCashHoldings].sort((a, b) => a.dailyChangePercent - b.dailyChangePercent).slice(0, 3),
    [nonCashHoldings]
  );

  const stats = useMemo(() => {
    if (!portfolio || filteredHoldings.length === 0) return null;
    const nonCash = filteredHoldings.filter((h) => h.assetType !== 'cash');
    if (nonCash.length === 0) {
      const cashTotal = filteredHoldings.reduce((sum, h) => sum + h.marketValueCad, 0);
      return { best: null, worst: null, cashTotal };
    }
    const best = nonCash.reduce(
      (a, b) => (b.gainLossPercent > a.gainLossPercent ? b : a),
      nonCash[0]!
    );
    const worst = nonCash.reduce(
      (a, b) => (b.gainLossPercent < a.gainLossPercent ? b : a),
      nonCash[0]!
    );
    const cashTotal = filteredHoldings
      .filter((h) => h.assetType === 'cash')
      .reduce((s, h) => s + h.marketValueCad, 0);
    return { best, worst, cashTotal };
  }, [filteredHoldings, portfolio]);

  // #49 — Account allocation data (always uses full portfolio, not filtered — #137)
  const accountData = useMemo(() => {
    if (!portfolio || portfolio.totalValue === 0) return [];
    const byAccount: Record<string, number> = {};
    for (const h of portfolio.holdings) {
      byAccount[h.account] = (byAccount[h.account] ?? 0) + h.marketValueCad;
    }
    return ACCOUNT_OPTIONS.filter((opt) => byAccount[opt.value] !== undefined).map((opt) => ({
      value: opt.value,
      label: opt.label,
      amount: byAccount[opt.value] ?? 0,
      pct: ((byAccount[opt.value] ?? 0) / portfolio.totalValue) * 100,
      isSelected: accountFilter !== 'all' && opt.value === accountFilter,
    }));
  }, [portfolio, accountFilter]);

  // #50 — Concentration risk data (non-cash only)
  const concentrationData = useMemo(() => {
    if (!portfolio || totals.totalValue === 0) return [];
    const nonCash = filteredHoldings.filter((h) => h.assetType !== 'cash');
    return [...nonCash]
      .sort((a, b) => b.weight - a.weight)
      .slice(0, config.topMoversCount)
      .map((h) => ({
        id: h.id,
        symbol: h.symbol,
        assetType: h.assetType,
        weightPct: h.weight, // already 0–100 percent from Rust
      }));
  }, [filteredHoldings, portfolio, totals]);

  const concentrationStats = useMemo(() => {
    if (concentrationData.length === 0) return null;
    const largest = concentrationData[0]?.weightPct ?? 0;
    const top3 = concentrationData.slice(0, 3).reduce((sum, d) => sum + d.weightPct, 0);
    return { largest, top3, hasRisk: largest > 20 };
  }, [concentrationData]);

  // #51 — Cash panel data
  const cashData = useMemo(() => {
    if (!portfolio) return { positions: [], totalCash: 0, cashPct: 0 };
    const cashHoldings = filteredHoldings.filter((h) => h.assetType === 'cash');
    const totalCash = cashHoldings.reduce((sum, h) => sum + h.marketValueCad, 0);
    const byCurrency: Record<string, number> = {};
    for (const h of cashHoldings) {
      byCurrency[h.currency] = (byCurrency[h.currency] ?? 0) + h.marketValueCad;
    }
    const positions = Object.entries(byCurrency).map(([ccy, amount]) => ({ ccy, amount }));
    const cashPct = totals.totalValue > 0 ? (totalCash / totals.totalValue) * 100 : 0;
    return { positions, totalCash, cashPct };
  }, [filteredHoldings, portfolio, totals]);

  if ((!portfolio || portfolio.holdings.length === 0) && !loading) {
    return <EmptyState message="Add your first holding to get started." />;
  }

  if (portfolio && accountFilter !== 'all' && filteredHoldings.length === 0 && !loading) {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
        <ActionCenter insights={actionInsights} />
        <div
          style={{
            ...PANEL,
            background: 'var(--bg-surface)',
            overflow: 'visible',
            position: 'relative',
            zIndex: 1,
          }}
        >
          <div style={LABEL}>
            {t('dashboard.portfolioValue')} ({baseCurrency})
          </div>
          <div style={{ width: 180, marginBottom: 12 }}>
            <Select
              value={accountFilter}
              onChange={(value) => setAccountFilter(value as 'all' | AccountType)}
              options={[
                { value: 'all', label: t('holdings.allAccounts') },
                ...ACCOUNT_OPTIONS.map((option) => ({ value: option.value, label: option.label })),
              ]}
            />
          </div>
        </div>
        <EmptyState message="No holdings in this account." />
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      <ActionCenter insights={actionInsights} />
      <div
        className="dashboard-grid"
        style={{
          gridTemplateRows: 'auto auto auto auto',
        }}
      >
        {/* Panel 1 — Portfolio Value (spans 2 cols) */}
        <div
          style={{
            ...PANEL,
            gridColumn: 'span 2',
            background: 'var(--bg-surface)',
            minHeight: 0,
            overflow: 'visible',
            position: 'relative',
            zIndex: 1,
          }}
        >
          <div style={LABEL}>
            {t('dashboard.portfolioValue')} ({baseCurrency})
          </div>
          <div style={{ width: 180, marginBottom: 12 }}>
            <Select
              value={accountFilter}
              onChange={(value) => setAccountFilter(value as 'all' | AccountType)}
              options={[
                { value: 'all', label: t('holdings.allAccounts') },
                ...ACCOUNT_OPTIONS.map((option) => ({ value: option.value, label: option.label })),
              ]}
            />
          </div>
          <div
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 48,
              fontWeight: 600,
              color: 'var(--text-primary)',
              lineHeight: 1.1,
              letterSpacing: '-0.02em',
            }}
          >
            {portfolio ? formatCurrency(totals.totalValue, baseCurrency) : '—'}
          </div>
          <div style={{ display: 'flex', gap: 24, marginTop: 10, alignItems: 'baseline' }}>
            <span
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 16,
                fontWeight: 600,
                color: pnlColor(totals.dailyPnl),
              }}
            >
              {portfolio
                ? `${totals.dailyPnl >= 0 ? '+' : ''}${formatCurrency(totals.dailyPnl, baseCurrency)}`
                : '—'}
            </span>
            {portfolio && (
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 13,
                  color: pnlColor(totals.dailyPnl),
                }}
              >
                {t('common.today')}
              </span>
            )}
          </div>
          <div
            style={{
              borderTop: '1px solid var(--border-subtle)',
              marginTop: 14,
              paddingTop: 14,
              display: 'flex',
              gap: 32,
            }}
          >
            <div>
              <div style={{ ...LABEL, marginBottom: 2 }}>
                {t('dashboard.costBasis')} ({baseCurrency})
              </div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 13,
                  color: 'var(--text-secondary)',
                }}
              >
                {portfolio ? formatCurrency(totals.totalCost, baseCurrency) : '—'}
              </div>
            </div>
            <div>
              <div style={{ ...LABEL, marginBottom: 2 }}>
                {t('dashboard.totalGainLoss')} ({baseCurrency})
              </div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 13,
                  color: pnlColor(totals.totalGainLoss),
                }}
              >
                {portfolio
                  ? `${totals.totalGainLoss >= 0 ? '+' : ''}${formatCurrency(totals.totalGainLoss, baseCurrency)} (${formatPercent(totals.totalGainLossPercent)})`
                  : '—'}
              </div>
            </div>
            <div>
              <div style={{ ...LABEL, marginBottom: 2 }}>{t('dashboard.holdings')}</div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 13,
                  color: 'var(--text-secondary)',
                }}
              >
                {portfolio
                  ? `${filteredHoldings.length} position${filteredHoldings.length !== 1 ? 's' : ''}`
                  : '—'}
              </div>
            </div>
            <div>
              <div style={{ ...LABEL, marginBottom: 2 }}>{t('dashboard.lastUpdated')}</div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 13,
                  color: 'var(--text-secondary)',
                }}
              >
                {portfolio
                  ? new Date(portfolio.lastUpdated).toLocaleTimeString([], {
                      hour: '2-digit',
                      minute: '2-digit',
                    })
                  : '—'}
              </div>
            </div>
            <div>
              <div style={{ ...LABEL, marginBottom: 2 }}>
                {t('dashboard.annualDividend')} ({baseCurrency})
              </div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 13,
                  color:
                    portfolio && portfolio.annualDividendIncome > 0
                      ? 'var(--color-gain)'
                      : 'var(--text-muted)',
                }}
              >
                {portfolio ? formatCurrency(portfolio.annualDividendIncome, baseCurrency) : '—'}
              </div>
            </div>
          </div>
        </div>

        {/* Panel 2 — Asset Allocation */}
        <div
          style={{
            ...PANEL,
            background: 'var(--bg-surface)',
            display: 'flex',
            flexDirection: 'column',
            minHeight: 0,
            overflow: 'hidden',
          }}
        >
          <div
            style={{
              flexShrink: 0,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              marginBottom: 8,
            }}
          >
            <div style={{ ...LABEL, marginBottom: 0 }}>
              {t('dashboard.allocation')} ({baseCurrency})
            </div>
            <div style={{ display: 'flex', gap: 0 }}>
              {(['type', 'account'] as const).map((view) => (
                <button
                  key={view}
                  onClick={() => setAllocView(view)}
                  style={{
                    padding: '2px 8px',
                    fontSize: 10,
                    fontFamily: 'var(--font-mono)',
                    textTransform: 'uppercase',
                    letterSpacing: '0.06em',
                    background: allocView === view ? 'var(--color-accent)' : 'transparent',
                    border: '1px solid var(--border-primary)',
                    borderLeft: view === 'account' ? 'none' : '1px solid var(--border-primary)',
                    color: allocView === view ? '#fff' : 'var(--text-muted)',
                    cursor: 'pointer',
                    borderRadius: 0,
                  }}
                >
                  {view === 'type' ? t('dashboard.assetType') : t('dashboard.account')}
                </button>
              ))}
            </div>
          </div>
          <div style={{ height: 200, flexShrink: 0 }}>
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Pie
                  data={allocView === 'type' ? allocationData : accountAllocationData}
                  cx="50%"
                  cy="50%"
                  innerRadius={50}
                  outerRadius={72}
                  dataKey="value"
                  strokeWidth={0}
                >
                  {(allocView === 'type' ? allocationData : accountAllocationData).map(
                    (entry, i) => (
                      <Cell key={i} fill={entry.color} />
                    )
                  )}
                </Pie>
                <CenterLabel text={allocView === 'type' ? 'Allocation' : 'Accounts'} />
                <Tooltip
                  contentStyle={{
                    background: 'var(--bg-surface)',
                    border: '1px solid var(--border-primary)',
                    borderRadius: 0,
                    fontSize: 12,
                    fontFamily: 'var(--font-mono)',
                  }}
                  formatter={(v: unknown) => [formatCurrency(Number(v), baseCurrency), '']}
                />
              </PieChart>
            </ResponsiveContainer>
          </div>
          <div
            style={{
              flexShrink: 0,
              display: 'flex',
              flexDirection: 'column',
              gap: 4,
              marginTop: 8,
            }}
          >
            {(allocView === 'type' ? allocationData : accountAllocationData).map((d) => (
              <div
                key={d.name}
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  fontSize: 11,
                }}
              >
                <span
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 6,
                    color: 'var(--text-secondary)',
                    fontFamily: 'var(--font-mono)',
                  }}
                >
                  <span
                    style={{
                      width: 8,
                      height: 8,
                      borderRadius: '50%',
                      background: d.color,
                      display: 'inline-block',
                    }}
                  />
                  {d.name}
                </span>
                <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--text-primary)' }}>
                  {formatNumber(d.pct, 1)}% · {formatCurrency(d.value, baseCurrency)}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Panel 3 — Top Gainers / Top Losers (spans 2 cols) — #339 */}
        <div
          style={{
            ...PANEL,
            gridColumn: 'span 2',
            background: 'var(--bg-surface)',
            display: 'flex',
            flexDirection: 'column',
            minHeight: 220,
            overflow: 'hidden',
          }}
        >
          <div
            style={{
              ...LABEL,
              flexShrink: 0,
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              marginBottom: 8,
            }}
          >
            <span>{topMoversTitle(portfolio?.lastUpdated)}</span>
            {portfolio && nonCashHoldings.length > 6 && (
              <button
                onClick={() => void navigate('/holdings')}
                style={{
                  background: 'none',
                  border: 'none',
                  color: 'var(--color-accent)',
                  fontFamily: 'var(--font-mono)',
                  fontSize: 11,
                  cursor: 'pointer',
                  padding: 0,
                  textTransform: 'uppercase',
                  letterSpacing: '0.06em',
                }}
              >
                {t('common.viewAll')}
              </button>
            )}
          </div>
          <div style={{ flex: 1, minHeight: 0, display: 'flex', gap: 16, overflow: 'hidden' }}>
            {(['gainers', 'losers'] as const).map((side) => {
              const movers = side === 'gainers' ? topGainers : topLosers;
              const label =
                side === 'gainers' ? t('dashboard.topGainers') : t('dashboard.topLosers');
              const accentColor = side === 'gainers' ? 'var(--color-gain)' : 'var(--color-loss)';
              return (
                <div key={side} style={{ flex: 1, minWidth: 0, overflowY: 'auto' }}>
                  <div
                    style={{
                      fontSize: 10,
                      color: accentColor,
                      fontFamily: 'var(--font-mono)',
                      textTransform: 'uppercase',
                      letterSpacing: '0.08em',
                      marginBottom: 6,
                      fontWeight: 600,
                    }}
                  >
                    {label}
                  </div>
                  <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 12 }}>
                    <thead>
                      <tr>
                        {['Symbol', 'Name', '%'].map((col) => (
                          <th
                            key={col}
                            style={{
                              textAlign: col === '%' ? 'right' : 'left',
                              padding: '4px 0',
                              color: 'var(--text-muted)',
                              fontWeight: 400,
                              fontFamily: 'var(--font-mono)',
                              fontSize: 10,
                              textTransform: 'uppercase',
                              letterSpacing: '0.06em',
                              borderBottom: '1px solid var(--border-primary)',
                            }}
                          >
                            {col}
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {movers.map((h) => (
                        <tr key={h.id} style={{ borderBottom: '1px solid var(--border-subtle)' }}>
                          <td
                            style={{
                              padding: '6px 0',
                              fontFamily: 'var(--font-mono)',
                              fontWeight: 600,
                              color: 'var(--text-primary)',
                              fontSize: 12,
                            }}
                          >
                            {h.symbol}
                          </td>
                          <td
                            style={{
                              padding: '6px 0',
                              color: 'var(--text-secondary)',
                              fontSize: 11,
                              overflow: 'hidden',
                              textOverflow: 'ellipsis',
                              whiteSpace: 'nowrap',
                              maxWidth: 120,
                            }}
                          >
                            {h.name}
                          </td>
                          <td
                            style={{
                              padding: '6px 0',
                              textAlign: 'right',
                              fontFamily: 'var(--font-mono)',
                              color: pnlColor(h.dailyChangePercent),
                              whiteSpace: 'nowrap',
                            }}
                          >
                            {h.dailyChangePercent > 0
                              ? '\u25b2'
                              : h.dailyChangePercent < 0
                                ? '\u25bc'
                                : ''}{' '}
                            {formatPercent(h.dailyChangePercent)}
                          </td>
                        </tr>
                      ))}
                      {movers.length === 0 && (
                        <tr>
                          <td
                            colSpan={3}
                            style={{
                              padding: '12px 0',
                              color: 'var(--text-muted)',
                              fontSize: 11,
                              textAlign: 'center',
                            }}
                          >
                            —
                          </td>
                        </tr>
                      )}
                    </tbody>
                  </table>
                </div>
              );
            })}
          </div>
        </div>

        {/* Panel 4 — Currency Exposure */}
        <div
          style={{
            ...PANEL,
            background: 'var(--bg-surface)',
            display: 'flex',
            flexDirection: 'column',
            minHeight: 0,
            overflow: 'hidden',
          }}
        >
          <div style={{ ...LABEL, flexShrink: 0 }}>
            {t('dashboard.currencyExposure')} ({baseCurrency} base)
          </div>
          <div style={{ height: 200, flexShrink: 0 }}>
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Pie
                  data={currencyData}
                  cx="50%"
                  cy="50%"
                  innerRadius={50}
                  outerRadius={72}
                  dataKey="value"
                  strokeWidth={0}
                >
                  {currencyData.map((entry, i) => (
                    <Cell key={i} fill={entry.color} />
                  ))}
                </Pie>
                <CenterLabel text="Currency" />
                <Tooltip
                  contentStyle={{
                    background: 'var(--bg-surface)',
                    border: '1px solid var(--border-primary)',
                    borderRadius: 0,
                    fontSize: 12,
                    fontFamily: 'var(--font-mono)',
                  }}
                  formatter={(v: unknown) => [formatCurrency(Number(v), baseCurrency), '']}
                />
              </PieChart>
            </ResponsiveContainer>
          </div>
          <div
            style={{
              flexShrink: 0,
              display: 'flex',
              flexDirection: 'column',
              gap: 4,
              marginTop: 8,
            }}
          >
            {currencyData.map((d) => (
              <div
                key={d.name}
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  fontSize: 11,
                }}
              >
                <span
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 6,
                    color: 'var(--text-secondary)',
                    fontFamily: 'var(--font-mono)',
                  }}
                >
                  <span
                    style={{
                      width: 8,
                      height: 8,
                      borderRadius: '50%',
                      background: d.color,
                      display: 'inline-block',
                    }}
                  />
                  {d.name}
                </span>
                <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--text-primary)' }}>
                  {formatNumber(d.pct, 1)}%
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Row 3 — By Account (#49), Concentration (#50), Cash (#51) */}

        {/* Panel 6 — By Account (#49) */}
        <div
          style={{
            ...PANEL,
            background: 'var(--bg-surface)',
            display: 'flex',
            flexDirection: 'column',
            minHeight: 0,
            overflow: 'hidden',
          }}
        >
          <div style={{ ...LABEL, flexShrink: 0 }}>{t('dashboard.byAccount')}</div>
          {accountData.length === 0 ? (
            <div
              style={{
                color: 'var(--text-muted)',
                fontFamily: 'var(--font-mono)',
                fontSize: 12,
                marginTop: 8,
              }}
            >
              {t('dashboard.noAccountData')}
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              {accountData.map((acct) => (
                <div key={acct.value}>
                  <div
                    style={{
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'baseline',
                      marginBottom: 4,
                    }}
                  >
                    <span
                      style={{
                        fontFamily: 'var(--font-mono)',
                        fontSize: 11,
                        color: acct.isSelected ? 'var(--color-accent)' : 'var(--text-secondary)',
                        textTransform: 'uppercase',
                        letterSpacing: '0.06em',
                        fontWeight: acct.isSelected ? 600 : 400,
                      }}
                    >
                      {acct.label}
                    </span>
                    <span
                      style={{
                        fontFamily: 'var(--font-mono)',
                        fontSize: 11,
                        color: acct.isSelected ? 'var(--text-primary)' : 'var(--text-secondary)',
                      }}
                    >
                      {formatCurrency(acct.amount, baseCurrency)}{' '}
                      <span style={{ color: 'var(--text-muted)' }}>
                        {formatNumber(acct.pct, 1)}%
                      </span>
                    </span>
                  </div>
                  <div
                    style={{
                      height: 4,
                      background: 'var(--bg-surface-alt)',
                      border: '1px solid var(--border-subtle)',
                    }}
                  >
                    <div
                      style={{
                        height: '100%',
                        width: `${acct.pct}%`,
                        background: acct.isSelected
                          ? 'var(--color-accent)'
                          : 'var(--border-primary)',
                      }}
                    />
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Panel 7 — Concentration (#50) */}
        <div
          style={{
            ...PANEL,
            background: 'var(--bg-surface)',
            display: 'flex',
            flexDirection: 'column',
            minHeight: 0,
            overflow: 'hidden',
          }}
        >
          <div style={{ ...LABEL, flexShrink: 0 }}>{t('dashboard.concentration')}</div>
          {concentrationData.length === 0 ? (
            <div
              style={{
                color: 'var(--text-muted)',
                fontFamily: 'var(--font-mono)',
                fontSize: 12,
                marginTop: 8,
              }}
            >
              {t('dashboard.noNonCashHoldings')}
            </div>
          ) : (
            <>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 5, flex: 1 }}>
                {concentrationData.map((h) => {
                  const assetColor =
                    ASSET_TYPE_CONFIG[h.assetType as keyof typeof ASSET_TYPE_CONFIG]?.color ??
                    '#888';
                  return (
                    <div
                      key={h.id}
                      style={{
                        display: 'flex',
                        justifyContent: 'space-between',
                        alignItems: 'center',
                        fontSize: 11,
                      }}
                    >
                      <span
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          gap: 6,
                          fontFamily: 'var(--font-mono)',
                          color: 'var(--text-primary)',
                          fontWeight: 600,
                        }}
                      >
                        <span
                          style={{
                            width: 7,
                            height: 7,
                            borderRadius: '50%',
                            background: assetColor,
                            display: 'inline-block',
                            flexShrink: 0,
                          }}
                        />
                        {h.symbol}
                      </span>
                      <span
                        style={{
                          fontFamily: 'var(--font-mono)',
                          color: 'var(--text-secondary)',
                          fontSize: 11,
                        }}
                      >
                        {formatNumber(h.weightPct, 1)}%
                      </span>
                    </div>
                  );
                })}
              </div>
              {concentrationStats && (
                <div
                  style={{
                    borderTop: '1px solid var(--border-subtle)',
                    marginTop: 10,
                    paddingTop: 10,
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 3,
                  }}
                >
                  <div
                    style={{
                      fontFamily: 'var(--font-mono)',
                      fontSize: 11,
                      color: 'var(--text-muted)',
                    }}
                  >
                    {t('dashboard.largest')}:{' '}
                    <span style={{ color: 'var(--text-secondary)' }}>
                      {formatNumber(concentrationStats.largest, 1)}%
                    </span>
                    {'  '}
                    {t('dashboard.top3')}:{' '}
                    <span style={{ color: 'var(--text-secondary)' }}>
                      {formatNumber(concentrationStats.top3, 1)}%
                    </span>
                  </div>
                  {concentrationStats.hasRisk && (
                    <div
                      style={{
                        fontFamily: 'var(--font-mono)',
                        fontSize: 11,
                        color: 'var(--color-warning)',
                      }}
                    >
                      &#9888; {t('dashboard.concentrationRisk')}
                    </div>
                  )}
                </div>
              )}
            </>
          )}
        </div>

        {/* Panel 8 — Cash (#51) */}
        <div
          style={{
            ...PANEL,
            background: 'var(--bg-surface)',
            display: 'flex',
            flexDirection: 'column',
            minHeight: 0,
            overflow: 'hidden',
          }}
        >
          <div style={{ ...LABEL, flexShrink: 0 }}>{t('dashboard.cash')}</div>
          {cashData.positions.length === 0 ? (
            <div
              style={{
                color: 'var(--text-muted)',
                fontFamily: 'var(--font-mono)',
                fontSize: 12,
                marginTop: 8,
              }}
            >
              {t('dashboard.noCashPositions')}
            </div>
          ) : (
            <>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 20,
                  fontWeight: 600,
                  color: 'var(--color-cash)',
                  marginBottom: 2,
                }}
              >
                {formatCurrency(cashData.totalCash, baseCurrency)}
              </div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 11,
                  color: 'var(--text-muted)',
                  marginBottom: 12,
                }}
              >
                {t('dashboard.investableCash')}
                <span style={{ marginLeft: 6, color: 'var(--text-secondary)' }}>
                  {t('dashboard.ofPortfolio', { pct: formatNumber(cashData.cashPct, 1) })}
                </span>
              </div>
              {cashData.positions.length > 1 && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 5 }}>
                  {cashData.positions.map((pos) => (
                    <div
                      key={pos.ccy}
                      style={{
                        display: 'flex',
                        justifyContent: 'space-between',
                        alignItems: 'center',
                        fontSize: 11,
                      }}
                    >
                      <span
                        style={{
                          fontFamily: 'var(--font-mono)',
                          color: 'var(--text-secondary)',
                          display: 'flex',
                          alignItems: 'center',
                          gap: 6,
                        }}
                      >
                        <span
                          style={{
                            width: 7,
                            height: 7,
                            borderRadius: '50%',
                            background: CURRENCY_COLORS[pos.ccy] ?? '#888',
                            display: 'inline-block',
                          }}
                        />
                        {pos.ccy} Cash
                      </span>
                      <span
                        style={{ fontFamily: 'var(--font-mono)', color: 'var(--text-primary)' }}
                      >
                        {formatCurrency(pos.amount, baseCurrency)}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </>
          )}
        </div>

        {/* Panel 5 — Quick Stats (full width) */}
        <div
          style={{
            ...PANEL,
            gridColumn: 'span 3',
            background: 'var(--bg-surface)',
            display: 'flex',
            gap: 0,
          }}
        >
          {[
            {
              label: t('dashboard.positions'),
              value: portfolio
                ? String(filteredHoldings.filter((h) => h.assetType !== 'cash').length)
                : '—',
              sub: portfolio ? `${filteredHoldings.length} ${t('common.total')}` : '',
            },
            {
              label: t('dashboard.bestPerformer'),
              value: stats?.best ? stats.best.symbol : '—',
              sub: stats?.best ? formatPercent(stats.best.gainLossPercent) : '',
              subColor: stats?.best ? pnlColor(stats.best.gainLossPercent) : undefined,
            },
            {
              label: t('dashboard.worstPerformer'),
              value: stats?.worst ? stats.worst.symbol : '—',
              sub: stats?.worst ? formatPercent(stats.worst.gainLossPercent) : '',
              subColor: stats?.worst ? pnlColor(stats.worst.gainLossPercent) : undefined,
            },
            {
              label: t('dashboard.cashPosition'),
              value: stats ? formatCompact(stats.cashTotal) : '—',
              sub:
                stats && totals.totalValue > 0
                  ? t('dashboard.ofPortfolio', {
                      pct: formatNumber((stats.cashTotal / totals.totalValue) * 100, 1),
                    })
                  : '',
            },
            {
              label: t('dashboard.realizedGains'),
              // When no cost-basis method has been chosen, suppress the value (#488)
              value: portfolio?.requiresCostBasisSelection
                ? '—'
                : portfolio
                  ? `${portfolio.realizedGains >= 0 ? '+' : ''}${formatCurrency(portfolio.realizedGains, baseCurrency)}`
                  : '—',
              sub: portfolio?.requiresCostBasisSelection ? '' : t('common.allTime'),
              subColor:
                portfolio && !portfolio.requiresCostBasisSelection
                  ? pnlColor(portfolio.realizedGains)
                  : undefined,
              valueColor:
                portfolio && !portfolio.requiresCostBasisSelection
                  ? pnlColor(portfolio.realizedGains)
                  : undefined,
              /** Show "Set method" link when cost-basis method has not been chosen (#488). */
              actionLabel: portfolio?.requiresCostBasisSelection ? 'Set method' : undefined,
              onAction:
                portfolio?.requiresCostBasisSelection && onOpenCostBasisModal
                  ? onOpenCostBasisModal
                  : undefined,
            },
          ].map((stat, i, arr) => (
            <div
              key={stat.label}
              style={{
                flex: 1,
                padding: '0 20px',
                borderRight: i < arr.length - 1 ? '1px solid var(--border-subtle)' : 'none',
              }}
            >
              <div style={LABEL}>{stat.label}</div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 20,
                  fontWeight: 600,
                  color:
                    'valueColor' in stat && stat.valueColor
                      ? stat.valueColor
                      : 'var(--text-primary)',
                }}
              >
                {stat.value}
              </div>
              {'actionLabel' in stat && stat.actionLabel && stat.onAction && (
                <button
                  onClick={stat.onAction}
                  style={{
                    background: 'none',
                    border: 'none',
                    color: 'var(--color-accent)',
                    cursor: 'pointer',
                    fontFamily: 'var(--font-mono)',
                    fontSize: 11,
                    padding: 0,
                    marginTop: 4,
                    textDecoration: 'underline',
                  }}
                >
                  {stat.actionLabel}
                </button>
              )}
              {stat.sub && (
                <div
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: 11,
                    color: stat.subColor ?? 'var(--text-muted)',
                    marginTop: 2,
                  }}
                >
                  {stat.sub}
                </div>
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
