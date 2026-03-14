import { useMemo, useState } from 'react';
import { PieChart, Pie, Cell, Tooltip, ResponsiveContainer } from 'recharts';
import { formatCurrency, formatCompact, formatPercent } from '../lib/format';
import { pnlColor } from '../lib/colors';
import { ACCOUNT_OPTIONS, ASSET_TYPE_CONFIG, CURRENCY_COLORS } from '../lib/constants';
import { EmptyState } from './ui/EmptyState';
import { Select } from './ui/Select';
import type { AccountType, PortfolioSnapshot } from '../types/portfolio';

interface DashboardProps {
  portfolio: PortfolioSnapshot | null;
  loading: boolean;
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

export function Dashboard({ portfolio, loading }: DashboardProps) {
  const [accountFilter, setAccountFilter] = useState<'all' | AccountType>('all');
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

  const topMovers = useMemo(() => {
    if (!portfolio) return [];
    return [...filteredHoldings]
      .filter((h) => h.assetType !== 'cash')
      .sort((a, b) => Math.abs(b.dailyChangePercent) - Math.abs(a.dailyChangePercent))
      .slice(0, 5);
  }, [filteredHoldings, portfolio]);

  const stats = useMemo(() => {
    if (!portfolio || filteredHoldings.length === 0) return null;
    const nonCash = filteredHoldings.filter((h) => h.assetType !== 'cash');
    if (nonCash.length === 0) {
      const cashTotal = filteredHoldings.reduce((sum, h) => sum + h.marketValueCad, 0);
      return { best: null, worst: null, cashTotal };
    }
    const best = nonCash.reduce(
      (a, b) => (b.gainLossPercent > a.gainLossPercent ? b : a),
      nonCash[0]
    );
    const worst = nonCash.reduce(
      (a, b) => (b.gainLossPercent < a.gainLossPercent ? b : a),
      nonCash[0]
    );
    const cashTotal = filteredHoldings
      .filter((h) => h.assetType === 'cash')
      .reduce((s, h) => s + h.marketValueCad, 0);
    return { best, worst, cashTotal };
  }, [filteredHoldings, portfolio]);

  if (!portfolio && !loading) {
    return <EmptyState message="No portfolio data available" />;
  }

  return (
    <div
      style={{
        display: 'grid',
        gridTemplateColumns: '1fr 1fr 1fr',
        gridTemplateRows: 'minmax(0, 1fr) minmax(0, 1fr) auto',
        gap: '1px',
        background: 'var(--border-primary)',
        border: '1px solid var(--border-primary)',
        height: '100%',
        minHeight: 0,
      }}
    >
      {/* Panel 1 — Portfolio Value (spans 2 cols) */}
      <div
        style={{
          ...PANEL,
          gridColumn: 'span 2',
          background: 'var(--bg-surface)',
          minHeight: 0,
          overflow: 'hidden',
        }}
      >
        <div style={LABEL}>Portfolio Value ({baseCurrency})</div>
        <div style={{ width: 180, marginBottom: 12 }}>
          <Select
            value={accountFilter}
            onChange={(value) => setAccountFilter(value as 'all' | AccountType)}
            options={[
              { value: 'all', label: 'All Accounts' },
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
              today
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
            <div style={{ ...LABEL, marginBottom: 2 }}>Cost Basis ({baseCurrency})</div>
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
            <div style={{ ...LABEL, marginBottom: 2 }}>Total Gain/Loss ({baseCurrency})</div>
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
        <div style={{ ...LABEL, flexShrink: 0 }}>Allocation ({baseCurrency})</div>
        <div style={{ flex: 1, minHeight: 0 }}>
          <ResponsiveContainer width="100%" height="100%">
            <PieChart>
              <Pie
                data={allocationData}
                cx="50%"
                cy="50%"
                innerRadius={50}
                outerRadius={72}
                dataKey="value"
                strokeWidth={0}
              >
                {allocationData.map((entry, i) => (
                  <Cell key={i} fill={entry.color} />
                ))}
              </Pie>
              <CenterLabel text="Allocation" />
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
          style={{ flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 4, marginTop: 8 }}
        >
          {allocationData.map((d) => (
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
                {d.pct.toFixed(1)}% · {formatCurrency(d.value, baseCurrency)}
              </span>
            </div>
          ))}
        </div>
      </div>

      {/* Panel 3 — Top Movers (spans 2 cols) */}
      <div
        style={{
          ...PANEL,
          gridColumn: 'span 2',
          background: 'var(--bg-surface)',
          display: 'flex',
          flexDirection: 'column',
          minHeight: 0,
          overflow: 'hidden',
        }}
      >
        <div style={{ ...LABEL, flexShrink: 0 }}>Top Movers</div>
        <div style={{ flex: 1, minHeight: 0, overflowY: 'auto' }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 12 }}>
            <thead>
              <tr>
                {['Symbol', 'Name', 'Change %', `Change (${baseCurrency})`].map((col) => (
                  <th
                    key={col}
                    style={{
                      textAlign: col === 'Change %' || col === 'Change $' ? 'right' : 'left',
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
              {topMovers.map((h) => {
                const dailyChange = h.marketValueCad * (h.dailyChangePercent / 100);
                return (
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
                    <td style={{ padding: '6px 0', color: 'var(--text-secondary)', fontSize: 11 }}>
                      {h.name}
                    </td>
                    <td
                      style={{
                        padding: '6px 0',
                        textAlign: 'right',
                        fontFamily: 'var(--font-mono)',
                        color: pnlColor(h.dailyChangePercent),
                      }}
                    >
                      {formatPercent(h.dailyChangePercent)}
                    </td>
                    <td
                      style={{
                        padding: '6px 0',
                        textAlign: 'right',
                        fontFamily: 'var(--font-mono)',
                        color: pnlColor(dailyChange),
                      }}
                    >
                      {dailyChange >= 0 ? '+' : ''}
                      {formatCurrency(dailyChange, baseCurrency)}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
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
        <div style={{ ...LABEL, flexShrink: 0 }}>Currency Exposure ({baseCurrency} base)</div>
        <div style={{ flex: 1, minHeight: 0 }}>
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
          style={{ flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 4, marginTop: 8 }}
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
                {d.pct.toFixed(1)}%
              </span>
            </div>
          ))}
        </div>
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
            label: 'Positions',
            value: portfolio
              ? String(filteredHoldings.filter((h) => h.assetType !== 'cash').length)
              : '—',
            sub: portfolio ? `${filteredHoldings.length} total` : '',
          },
          {
            label: 'Best Performer',
            value: stats?.best ? stats.best.symbol : '—',
            sub: stats?.best ? formatPercent(stats.best.gainLossPercent) : '',
            subColor: stats?.best ? pnlColor(stats.best.gainLossPercent) : undefined,
          },
          {
            label: 'Worst Performer',
            value: stats?.worst ? stats.worst.symbol : '—',
            sub: stats?.worst ? formatPercent(stats.worst.gainLossPercent) : '',
            subColor: stats?.worst ? pnlColor(stats.worst.gainLossPercent) : undefined,
          },
          {
            label: 'Cash Position',
            value: stats ? formatCompact(stats.cashTotal) : '—',
            sub:
              stats && totals.totalValue > 0
                ? `${((stats.cashTotal / totals.totalValue) * 100).toFixed(1)}% of portfolio`
                : '',
          },
          {
            label: 'Portfolio Beta',
            value: '—',
            sub: 'v2 feature',
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
                color: 'var(--text-primary)',
              }}
            >
              {stat.value}
            </div>
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
  );
}
