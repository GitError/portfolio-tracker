import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw, ArrowUpDown } from 'lucide-react';
import { isTauri, tauriInvoke } from '../lib/tauri';
import {
  PieChart,
  Pie,
  Cell,
  Tooltip,
  Legend,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  ResponsiveContainer,
} from 'recharts';
import { Spinner } from './ui/Spinner';
import { EmptyState } from './ui/EmptyState';
import { formatNumber, formatPercent } from '../lib/format';
import type { PortfolioAnalytics, SymbolMetadata } from '../types/portfolio';

// Chart color palette using design tokens + extended palette for extra sectors
const SECTOR_COLORS = [
  'var(--color-stock)',
  'var(--color-etf)',
  'var(--color-crypto)',
  'var(--color-cash)',
  '#ec4899',
  '#14b8a6',
  '#f97316',
  '#a855f7',
  '#06b6d4',
  '#84cc16',
];

type SortKey = keyof SymbolMetadata;
type SortDir = 'asc' | 'desc';

function RiskCard({ label, value, sub }: { label: string; value: string; sub?: string }) {
  return (
    <div
      style={{
        background: 'var(--bg-surface)',
        border: '1px solid var(--border-primary)',
        borderRadius: 2,
        padding: '16px 20px',
        flex: '1 1 0',
        minWidth: 0,
      }}
    >
      <div
        style={{
          fontSize: 11,
          color: 'var(--text-secondary)',
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
          marginBottom: 8,
        }}
      >
        {label}
      </div>
      <div
        style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 22,
          fontWeight: 600,
          color: 'var(--text-primary)',
          lineHeight: 1,
        }}
      >
        {value}
      </div>
      {sub && (
        <div
          style={{
            fontSize: 11,
            color: 'var(--text-muted)',
            marginTop: 6,
          }}
        >
          {sub}
        </div>
      )}
    </div>
  );
}

function SortableHeader({
  label,
  sortKey,
  currentKey,
  currentDir,
  onSort,
}: {
  label: string;
  sortKey: SortKey;
  currentKey: SortKey | null;
  currentDir: SortDir;
  onSort: (key: SortKey) => void;
}) {
  const isActive = currentKey === sortKey;
  return (
    <th
      onClick={() => onSort(sortKey)}
      style={{
        cursor: 'pointer',
        padding: '10px 12px',
        textAlign: 'left',
        fontSize: 11,
        fontWeight: 600,
        color: isActive ? 'var(--text-primary)' : 'var(--text-secondary)',
        textTransform: 'uppercase',
        letterSpacing: '0.05em',
        userSelect: 'none',
        whiteSpace: 'nowrap',
        background: 'var(--bg-surface)',
        borderBottom: '1px solid var(--border-primary)',
      }}
    >
      <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
        {label}
        <ArrowUpDown
          size={11}
          style={{
            opacity: isActive ? 1 : 0.4,
            color: isActive ? 'var(--color-accent)' : 'inherit',
          }}
        />
        {isActive && (
          <span style={{ fontSize: 10, color: 'var(--color-accent)' }}>
            {currentDir === 'asc' ? '↑' : '↓'}
          </span>
        )}
      </span>
    </th>
  );
}

function MetaCell({ value }: { value: string | number | undefined }) {
  if (value == null) {
    return (
      <td
        style={{
          padding: '10px 12px',
          fontFamily: 'var(--font-mono)',
          fontSize: 13,
          color: 'var(--text-muted)',
          borderBottom: '1px solid var(--border-subtle)',
        }}
      >
        —
      </td>
    );
  }
  return (
    <td
      style={{
        padding: '10px 12px',
        fontFamily: 'var(--font-mono)',
        fontSize: 13,
        color: 'var(--text-primary)',
        borderBottom: '1px solid var(--border-subtle)',
      }}
    >
      {value}
    </td>
  );
}

function formatMarketCap(cap?: number, currency = 'USD'): string {
  if (cap == null) return '';
  const fmt = (value: number, suffix: string) => {
    const locale = navigator.language || 'en';
    const formatted = new Intl.NumberFormat(locale, {
      style: 'decimal',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(value);
    return `${formatted} ${currency}${suffix}`;
  };
  if (cap >= 1e12) return fmt(cap / 1e12, 'T');
  if (cap >= 1e9) return fmt(cap / 1e9, 'B');
  if (cap >= 1e6) return fmt(cap / 1e6, 'M');
  const locale = navigator.language || 'en';
  return `${new Intl.NumberFormat(locale, { style: 'decimal', minimumFractionDigits: 0, maximumFractionDigits: 0 }).format(cap)} ${currency}`;
}

export function Analytics() {
  const { t } = useTranslation();
  const [analytics, setAnalytics] = useState<PortfolioAnalytics | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sortKey, setSortKey] = useState<SortKey | null>(null);
  const [sortDir, setSortDir] = useState<SortDir>('asc');

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      if (isTauri()) {
        const result = await tauriInvoke<PortfolioAnalytics>('get_portfolio_analytics');
        setAnalytics(result);
      } else {
        // Browser dev mode: analytics require live data from Tauri
        setAnalytics({
          metadata: [],
          sectorBreakdown: [],
          countryBreakdown: [],
          riskMetrics: {
            portfolioYield: 0,
            concentrationHhi: 0,
            largestPositionWeight: 0,
          },
        });
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const handleSort = useCallback(
    (key: SortKey) => {
      if (sortKey === key) {
        setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
      } else {
        setSortKey(key);
        setSortDir('asc');
      }
    },
    [sortKey]
  );

  const sortedMetadata = analytics
    ? [...analytics.metadata].sort((a, b) => {
        if (!sortKey) return 0;
        const av = a[sortKey];
        const bv = b[sortKey];
        if (av == null && bv == null) return 0;
        if (av == null) return 1;
        if (bv == null) return -1;
        const cmp =
          typeof av === 'string' ? av.localeCompare(bv as string) : (av as number) - (bv as number);
        return sortDir === 'asc' ? cmp : -cmp;
      })
    : [];

  const pieData =
    analytics?.sectorBreakdown.map((s) => ({
      name: s.sector,
      value: parseFloat(s.weightPercent.toFixed(2)),
    })) ?? [];

  const barData =
    analytics?.countryBreakdown.map((c) => ({
      name: c.country,
      value: parseFloat(c.weightPercent.toFixed(2)),
    })) ?? [];

  // Only show geographic breakdown when at least 50% of non-cash holdings have
  // country data. The v7 quote endpoint does not reliably return country, so we
  // hide the chart until the data quality is sufficient.
  const showGeographicBreakdown = (() => {
    if (!analytics || analytics.metadata.length === 0) return false;
    const withCountry = analytics.metadata.filter(
      (m) => m.country != null && m.country !== ''
    ).length;
    return withCountry / analytics.metadata.length >= 0.5;
  })();

  return (
    <div
      style={{
        padding: '24px 28px',
        maxWidth: 1200,
        margin: '0 auto',
        color: 'var(--text-primary)',
      }}
    >
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          marginBottom: 24,
        }}
      >
        <div>
          <h1 style={{ margin: 0, fontSize: 20, fontWeight: 600 }}>{t('analytics.title')}</h1>
          <p
            style={{
              margin: '4px 0 0',
              fontSize: 13,
              color: 'var(--text-secondary)',
            }}
          >
            Sector breakdown and risk metrics
          </p>
        </div>
        <button
          onClick={() => void load()}
          disabled={loading}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            padding: '8px 14px',
            background: 'var(--color-accent)',
            color: '#fff',
            border: 'none',
            borderRadius: 2,
            cursor: loading ? 'not-allowed' : 'pointer',
            fontSize: 13,
            fontWeight: 500,
            opacity: loading ? 0.7 : 1,
          }}
        >
          <RefreshCw
            size={14}
            style={{ animation: loading ? 'spin 1s linear infinite' : 'none' }}
          />
          {analytics ? t('common.refresh') : 'Load Analytics'}
        </button>
      </div>

      {/* Error */}
      {error && (
        <div
          style={{
            background: 'rgba(255,71,87,0.1)',
            border: '1px solid var(--color-loss)',
            borderRadius: 2,
            padding: '12px 16px',
            marginBottom: 20,
            fontSize: 13,
            color: 'var(--color-loss)',
          }}
        >
          {error}
        </div>
      )}

      {/* Loading spinner */}
      {loading && !analytics && (
        <div
          style={{
            display: 'flex',
            justifyContent: 'center',
            paddingTop: 60,
          }}
        >
          <Spinner />
        </div>
      )}

      {/* Empty state — not yet loaded */}
      {!loading && !analytics && !error && (
        <EmptyState message="No analytics loaded — click 'Load Analytics' to fetch sector and risk data." />
      )}

      {/* No holdings */}
      {analytics && analytics.metadata.length === 0 && analytics.sectorBreakdown.length === 0 && (
        <EmptyState message="No holdings found — add holdings to your portfolio to see analytics." />
      )}

      {analytics && (analytics.metadata.length > 0 || analytics.sectorBreakdown.length > 0) && (
        <>
          {/* ── Section 1: Risk Metric Cards ── */}
          <section style={{ marginBottom: 32 }}>
            <h2
              style={{
                fontSize: 13,
                fontWeight: 600,
                color: 'var(--text-secondary)',
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                margin: '0 0 12px',
              }}
            >
              Risk Metrics
            </h2>
            <div style={{ display: 'flex', gap: 12 }}>
              <RiskCard
                label="Portfolio Beta"
                value={
                  analytics.riskMetrics.weightedBeta != null
                    ? analytics.riskMetrics.weightedBeta.toFixed(2)
                    : 'N/A'
                }
                sub="Weighted average beta"
              />
              <RiskCard
                label="Dividend Yield"
                value={
                  analytics.riskMetrics.portfolioYield > 0
                    ? `${(analytics.riskMetrics.portfolioYield * 100).toFixed(2)}%`
                    : '0.00%'
                }
                sub="Weighted portfolio yield"
              />
              <RiskCard
                label="HHI Concentration"
                value={formatNumber(analytics.riskMetrics.concentrationHhi, 0)}
                sub="Lower is more diversified"
              />
              <RiskCard
                label="Largest Position"
                value={`${analytics.riskMetrics.largestPositionWeight.toFixed(2)}%`}
                sub={analytics.riskMetrics.topSector ?? ''}
              />
            </div>
          </section>

          {/* ── Section 2 & 3: Charts row ── */}
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: showGeographicBreakdown ? '1fr 1fr' : '1fr',
              gap: 20,
              marginBottom: 32,
            }}
          >
            {/* Sector Pie */}
            <section
              style={{
                background: 'var(--bg-surface)',
                border: '1px solid var(--border-primary)',
                borderRadius: 2,
                padding: '20px',
              }}
            >
              <h2
                style={{
                  fontSize: 13,
                  fontWeight: 600,
                  color: 'var(--text-secondary)',
                  textTransform: 'uppercase',
                  letterSpacing: '0.05em',
                  margin: '0 0 16px',
                }}
              >
                Sector Breakdown
              </h2>
              {pieData.length > 0 ? (
                <ResponsiveContainer width="100%" height={260}>
                  <PieChart>
                    <Pie
                      data={pieData}
                      dataKey="value"
                      nameKey="name"
                      cx="50%"
                      cy="50%"
                      outerRadius={90}
                      label={({ name, value }) => `${name} ${(value as number).toFixed(1)}%`}
                      labelLine={false}
                    >
                      {pieData.map((_, index) => (
                        <Cell key={index} fill={SECTOR_COLORS[index % SECTOR_COLORS.length]!} />
                      ))}
                    </Pie>
                    <Tooltip
                      formatter={(value) => [`${(value as number).toFixed(2)}%`, 'Weight']}
                      contentStyle={{
                        background: 'var(--bg-surface)',
                        border: '1px solid var(--border-primary)',
                        borderRadius: 2,
                        fontSize: 12,
                        color: 'var(--text-primary)',
                      }}
                    />
                    <Legend wrapperStyle={{ fontSize: 12, color: 'var(--text-secondary)' }} />
                  </PieChart>
                </ResponsiveContainer>
              ) : (
                <div
                  style={{
                    height: 260,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    color: 'var(--text-muted)',
                    fontSize: 13,
                  }}
                >
                  No sector data available
                </div>
              )}
            </section>

            {/* Country Bar — only shown when >= 50% of holdings have country data */}
            {showGeographicBreakdown && (
              <section
                style={{
                  background: 'var(--bg-surface)',
                  border: '1px solid var(--border-primary)',
                  borderRadius: 2,
                  padding: '20px',
                }}
              >
                <h2
                  style={{
                    fontSize: 13,
                    fontWeight: 600,
                    color: 'var(--text-secondary)',
                    textTransform: 'uppercase',
                    letterSpacing: '0.05em',
                    margin: '0 0 16px',
                  }}
                >
                  Geographic Breakdown
                </h2>
                {barData.length > 0 ? (
                  <ResponsiveContainer width="100%" height={260}>
                    <BarChart
                      data={barData}
                      layout="vertical"
                      margin={{ top: 0, right: 20, left: 10, bottom: 0 }}
                    >
                      <CartesianGrid
                        strokeDasharray="3 3"
                        stroke="var(--border-subtle)"
                        horizontal={false}
                      />
                      <XAxis
                        type="number"
                        tickFormatter={(v: number) => `${v}%`}
                        tick={{ fontSize: 11, fill: 'var(--text-secondary)' }}
                        axisLine={{ stroke: 'var(--border-primary)' }}
                        tickLine={false}
                      />
                      <YAxis
                        type="category"
                        dataKey="name"
                        tick={{ fontSize: 11, fill: 'var(--text-secondary)' }}
                        axisLine={false}
                        tickLine={false}
                        width={90}
                      />
                      <Tooltip
                        formatter={(value) => [`${(value as number).toFixed(2)}%`, 'Weight']}
                        contentStyle={{
                          background: 'var(--bg-surface)',
                          border: '1px solid var(--border-primary)',
                          borderRadius: 2,
                          fontSize: 12,
                          color: 'var(--text-primary)',
                        }}
                        cursor={{ fill: 'var(--bg-surface-hover)' }}
                      />
                      <Bar dataKey="value" fill="var(--color-accent)" radius={0} />
                    </BarChart>
                  </ResponsiveContainer>
                ) : (
                  <div
                    style={{
                      height: 260,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      color: 'var(--text-muted)',
                      fontSize: 13,
                    }}
                  >
                    No geographic data available
                  </div>
                )}
              </section>
            )}
          </div>

          {/* ── Section 4: Holdings Detail Table ── */}
          <section>
            <h2
              style={{
                fontSize: 13,
                fontWeight: 600,
                color: 'var(--text-secondary)',
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                margin: '0 0 12px',
              }}
            >
              Holdings Detail
            </h2>
            <div
              style={{
                background: 'var(--bg-surface)',
                border: '1px solid var(--border-primary)',
                borderRadius: 0,
                overflowX: 'auto',
              }}
            >
              <table style={{ width: '100%', borderCollapse: 'collapse' }}>
                <thead>
                  <tr>
                    <SortableHeader
                      label="Symbol"
                      sortKey="symbol"
                      currentKey={sortKey}
                      currentDir={sortDir}
                      onSort={handleSort}
                    />
                    <SortableHeader
                      label="Sector"
                      sortKey="sector"
                      currentKey={sortKey}
                      currentDir={sortDir}
                      onSort={handleSort}
                    />
                    <SortableHeader
                      label="Industry"
                      sortKey="industry"
                      currentKey={sortKey}
                      currentDir={sortDir}
                      onSort={handleSort}
                    />
                    <SortableHeader
                      label="Country"
                      sortKey="country"
                      currentKey={sortKey}
                      currentDir={sortDir}
                      onSort={handleSort}
                    />
                    <SortableHeader
                      label="Beta"
                      sortKey="beta"
                      currentKey={sortKey}
                      currentDir={sortDir}
                      onSort={handleSort}
                    />
                    <SortableHeader
                      label="P/E"
                      sortKey="peRatio"
                      currentKey={sortKey}
                      currentDir={sortDir}
                      onSort={handleSort}
                    />
                    <SortableHeader
                      label="Div Yield"
                      sortKey="dividendYield"
                      currentKey={sortKey}
                      currentDir={sortDir}
                      onSort={handleSort}
                    />
                    <SortableHeader
                      label="Market Cap"
                      sortKey="marketCap"
                      currentKey={sortKey}
                      currentDir={sortDir}
                      onSort={handleSort}
                    />
                  </tr>
                </thead>
                <tbody>
                  {sortedMetadata.map((meta, i) => (
                    <tr
                      key={meta.symbol}
                      style={{
                        background: i % 2 === 0 ? 'var(--bg-surface)' : 'var(--bg-surface-alt)',
                        transition: 'background 100ms',
                      }}
                      onMouseEnter={(e) => {
                        (e.currentTarget as HTMLTableRowElement).style.background =
                          'var(--bg-surface-hover)';
                      }}
                      onMouseLeave={(e) => {
                        (e.currentTarget as HTMLTableRowElement).style.background =
                          i % 2 === 0 ? 'var(--bg-surface)' : 'var(--bg-surface-alt)';
                      }}
                    >
                      <td
                        style={{
                          padding: '10px 12px',
                          fontFamily: 'var(--font-mono)',
                          fontSize: 13,
                          fontWeight: 600,
                          color: 'var(--color-accent)',
                          borderBottom: '1px solid var(--border-subtle)',
                        }}
                      >
                        {meta.symbol}
                      </td>
                      <MetaCell value={meta.sector} />
                      <MetaCell value={meta.industry} />
                      <MetaCell value={meta.country} />
                      <MetaCell value={meta.beta != null ? meta.beta.toFixed(2) : undefined} />
                      <MetaCell
                        value={meta.peRatio != null ? meta.peRatio.toFixed(2) : undefined}
                      />
                      <MetaCell
                        value={
                          meta.dividendYield != null
                            ? formatPercent(meta.dividendYield * 100)
                            : undefined
                        }
                      />
                      <MetaCell
                        value={meta.marketCap != null ? formatMarketCap(meta.marketCap) : undefined}
                      />
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        </>
      )}
    </div>
  );
}
