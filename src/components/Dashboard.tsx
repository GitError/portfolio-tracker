import { useMemo, useState } from 'react';
import { PieChart, Pie, Cell, Tooltip, ResponsiveContainer } from 'recharts';
import { formatCurrency, formatCompact, formatPercent } from '../lib/format';
import { pnlColor } from '../lib/colors';
import { ACCOUNT_OPTIONS, ASSET_TYPE_CONFIG, CURRENCY_COLORS } from '../lib/constants';
import { EmptyState } from './ui/EmptyState';
import { config } from '../lib/config';
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
      .slice(0, config.topMoversCount);
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

  // #49 — Account allocation data
  const accountData = useMemo(() => {
    if (!portfolio || totals.totalValue === 0) return [];
    const byAccount: Record<string, number> = {};
    for (const h of filteredHoldings) {
      byAccount[h.account] = (byAccount[h.account] ?? 0) + h.marketValueCad;
    }
    return ACCOUNT_OPTIONS.filter((opt) => byAccount[opt.value] !== undefined).map((opt) => ({
      value: opt.value,
      label: opt.label,
      amount: byAccount[opt.value] ?? 0,
      pct: ((byAccount[opt.value] ?? 0) / totals.totalValue) * 100,
    }));
  }, [filteredHoldings, portfolio, totals]);

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
        weightPct: h.weight * 100,
      }));
  }, [filteredHoldings, portfolio, totals]);

  const concentrationStats = useMemo(() => {
    if (concentrationData.length === 0) return null;
    const largest = concentrationData[0].weightPct;
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

  return (
    <div
      style={{
        display: 'grid',
        gridTemplateColumns: '1fr 1fr 1fr',
        gridTemplateRows: 'minmax(0, 1fr) minmax(0, 1fr) minmax(0, 1fr) auto',
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

      {/* Panel 3 — Top Movers (spans 2 cols) — #45 */}
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
        <div style={{ ...LABEL, flexShrink: 0 }}>{topMoversTitle(portfolio?.lastUpdated)}</div>
        <div style={{ flex: 1, minHeight: 0, overflowY: 'auto' }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 12 }}>
            <thead>
              <tr>
                {['Symbol', 'Name', 'Change %', `Change (${baseCurrency})`].map((col) => (
                  <th
                    key={col}
                    style={{
                      textAlign:
                        col === 'Change %' || col === `Change (${baseCurrency})` ? 'right' : 'left',
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
                const arrow =
                  h.dailyChangePercent > 0 ? '\u25b2' : h.dailyChangePercent < 0 ? '\u25bc' : '';
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
                      {arrow && (
                        <span style={{ marginRight: 3, color: pnlColor(h.dailyChangePercent) }}>
                          {arrow}
                        </span>
                      )}
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
        <div style={{ ...LABEL, flexShrink: 0 }}>By Account</div>
        {accountData.length === 0 ? (
          <div
            style={{
              color: 'var(--text-muted)',
              fontFamily: 'var(--font-mono)',
              fontSize: 12,
              marginTop: 8,
            }}
          >
            No account data
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
                      color: 'var(--text-secondary)',
                      textTransform: 'uppercase',
                      letterSpacing: '0.06em',
                    }}
                  >
                    {acct.label}
                  </span>
                  <span
                    style={{
                      fontFamily: 'var(--font-mono)',
                      fontSize: 11,
                      color: 'var(--text-primary)',
                    }}
                  >
                    {formatCurrency(acct.amount, baseCurrency)}{' '}
                    <span style={{ color: 'var(--text-muted)' }}>{acct.pct.toFixed(1)}%</span>
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
                      background: 'var(--color-accent)',
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
        <div style={{ ...LABEL, flexShrink: 0 }}>Concentration</div>
        {concentrationData.length === 0 ? (
          <div
            style={{
              color: 'var(--text-muted)',
              fontFamily: 'var(--font-mono)',
              fontSize: 12,
              marginTop: 8,
            }}
          >
            No non-cash holdings
          </div>
        ) : (
          <>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 5, flex: 1 }}>
              {concentrationData.map((h) => {
                const assetColor =
                  ASSET_TYPE_CONFIG[h.assetType as keyof typeof ASSET_TYPE_CONFIG]?.color ?? '#888';
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
                      {h.weightPct.toFixed(1)}%
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
                  Largest:{' '}
                  <span style={{ color: 'var(--text-secondary)' }}>
                    {concentrationStats.largest.toFixed(1)}%
                  </span>
                  {'  '}Top 3:{' '}
                  <span style={{ color: 'var(--text-secondary)' }}>
                    {concentrationStats.top3.toFixed(1)}%
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
                    &#9888; Concentration risk
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
        <div style={{ ...LABEL, flexShrink: 0 }}>Cash</div>
        {cashData.positions.length === 0 ? (
          <div
            style={{
              color: 'var(--text-muted)',
              fontFamily: 'var(--font-mono)',
              fontSize: 12,
              marginTop: 8,
            }}
          >
            No cash positions
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
              Investable Cash
              <span style={{ marginLeft: 6, color: 'var(--text-secondary)' }}>
                {cashData.cashPct.toFixed(1)}% of portfolio
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
                    <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--text-primary)' }}>
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
