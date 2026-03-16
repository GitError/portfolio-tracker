import { useMemo, useState, useEffect } from 'react';
import {
  AreaChart,
  Area,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from 'recharts';
import { useSearchParams } from 'react-router-dom';
import { ALL_PERF_DATA, BENCHMARK_SERIES, calcStats, filterByRange } from '../lib/perfMockData';
import type { PerfDataPoint } from '../lib/perfMockData';
import { formatCurrency, formatCompact, formatPercent } from '../lib/format';
import { pnlColor } from '../lib/colors';
import { ACCOUNT_OPTIONS, ASSET_TYPE_CONFIG } from '../lib/constants';
import { Select } from './ui/Select';
import { EmptyState } from './ui/EmptyState';
import type { AccountType, AssetType, PortfolioSnapshot } from '../types/portfolio';

const isTauri = (): boolean => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

interface PerformancePoint {
  date: string;
  value: number;
}

function backendPointsToPerfData(points: PerformancePoint[]): PerfDataPoint[] {
  return points.map((point, index) => {
    const prev = index > 0 ? points[index - 1].value : point.value;
    const dailyReturn = prev !== 0 ? ((point.value - prev) / prev) * 100 : 0;
    return { date: point.date, value: point.value, dailyReturn };
  });
}

function useRealPerformance(range: string): {
  data: PerfDataPoint[];
  loading: boolean;
  isEmpty: boolean;
} {
  const [rawPoints, setRawPoints] = useState<PerformancePoint[] | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!isTauri()) return;
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setLoading(true);
    tauriInvoke<PerformancePoint[]>('get_performance', { range })
      .then((points) => setRawPoints(points))
      .catch((err) => {
        // eslint-disable-next-line no-console
        console.error('get_performance failed:', err);
        setRawPoints([]);
      })
      .finally(() => setLoading(false));
  }, [range]);

  const data = useMemo(
    () => (rawPoints !== null ? backendPointsToPerfData(rawPoints) : []),
    [rawPoints]
  );

  return {
    data,
    loading,
    isEmpty: isTauri() && rawPoints !== null && rawPoints.length === 0,
  };
}

type Range = '1D' | '1W' | '1M' | '3M' | '6M' | '1Y' | 'ALL';
const RANGES: Range[] = ['1D', '1W', '1M', '3M', '6M', '1Y', 'ALL'];
type AssetFilter = 'all' | AssetType;
type BenchmarkId = 'none' | 'sp500' | 'nasdaq100' | 'tsx' | 'bitcoin';

const PANEL: React.CSSProperties = {
  background: 'var(--bg-surface)',
  border: '1px solid var(--border-primary)',
  padding: '20px',
};

interface PerformanceProps {
  portfolio: PortfolioSnapshot | null;
  onRefresh?: () => void;
}

const STAT_LABEL: React.CSSProperties = {
  fontSize: 10,
  color: 'var(--text-muted)',
  fontFamily: 'var(--font-mono)',
  textTransform: 'uppercase',
  letterSpacing: '0.08em',
  marginBottom: 4,
};

function formatAxisDate(dateStr: string, range: Range): string {
  const d = new Date(dateStr + 'T00:00:00');
  if (range === '1D' || range === '1W') {
    return d.toLocaleDateString('en-CA', { month: 'short', day: 'numeric' });
  }
  if (range === '1M' || range === '3M') {
    return d.toLocaleDateString('en-CA', { month: 'short', day: 'numeric' });
  }
  return d.toLocaleDateString('en-CA', { month: 'short', year: '2-digit' });
}

function CustomTooltip({
  active,
  payload,
  label,
  currency,
}: {
  active?: boolean;
  payload?: Array<{ value: number; payload: { dailyReturn: number } }>;
  label?: string;
  currency: string;
}) {
  if (!active || !payload?.length) return null;
  const val = payload[0].value;
  const daily = payload[0].payload.dailyReturn;
  return (
    <div
      style={{
        background: 'var(--bg-surface)',
        border: '1px solid var(--border-primary)',
        padding: '10px 14px',
        fontSize: 12,
        fontFamily: 'var(--font-mono)',
      }}
    >
      <div style={{ color: 'var(--text-secondary)', marginBottom: 6 }}>{label}</div>
      <div style={{ color: 'var(--text-primary)', fontWeight: 600 }}>
        {formatCurrency(val, currency)}
      </div>
      <div style={{ color: pnlColor(daily), marginTop: 2 }}>{formatPercent(daily)} daily</div>
    </div>
  );
}

export function Performance({ portfolio, onRefresh }: PerformanceProps) {
  const [searchParams, setSearchParams] = useSearchParams();
  const range = (searchParams.get('range') as Range) || '1Y';
  const accountFilter = (searchParams.get('account') as 'all' | AccountType) || 'all';
  const assetFilter = (searchParams.get('asset') as AssetFilter) || 'all';
  const benchmarkId = (searchParams.get('benchmark') as BenchmarkId) || 'none';
  const baseCurrency = portfolio?.baseCurrency ?? 'CAD';

  function updateParam(key: string, value: string) {
    const next = new URLSearchParams(searchParams);
    if (!value || value === 'all' || value === 'none' || (key === 'range' && value === '1Y')) {
      next.delete(key);
    } else {
      next.set(key, value);
    }
    setSearchParams(next, { replace: true });
  }

  const filteredHoldings = useMemo(() => {
    if (!portfolio) return [];
    return portfolio.holdings.filter((holding) => {
      if (accountFilter !== 'all' && holding.account !== accountFilter) return false;
      if (assetFilter !== 'all' && holding.assetType !== assetFilter) return false;
      return true;
    });
  }, [portfolio, accountFilter, assetFilter]);

  const filteredValue = useMemo(
    () => filteredHoldings.reduce((sum, holding) => sum + holding.marketValueCad, 0),
    [filteredHoldings]
  );

  const filteredShare = useMemo(() => {
    if (!portfolio || portfolio.totalValue === 0) return 0;
    return filteredValue / portfolio.totalValue;
  }, [portfolio, filteredValue]);

  // Real performance data (Tauri) or scaled mock data (browser)
  const { data: realData, loading: perfLoading, isEmpty: perfIsEmpty } = useRealPerformance(range);

  const scaledMockData = useMemo(
    () =>
      ALL_PERF_DATA.map((point) => ({
        ...point,
        value: Math.round(point.value * filteredShare * 100) / 100,
      })),
    [filteredShare]
  );

  const data = useMemo(() => {
    if (isTauri()) {
      return realData;
    }
    return filterByRange(scaledMockData, range);
  }, [realData, scaledMockData, range]);

  const stats = useMemo(() => calcStats(data), [data]);
  const benchmark = useMemo(
    () => BENCHMARK_SERIES.find((series) => series.id === benchmarkId) ?? null,
    [benchmarkId]
  );
  const benchmarkData = useMemo(() => {
    if (!benchmark || data.length === 0) return [];
    const raw = filterByRange(benchmark.points, range);
    if (raw.length === 0) return [];
    const first = raw[0].value || 1;
    const base = data[0]?.value ?? 0;
    return raw.map((point) => ({
      ...point,
      value: Math.round((point.value / first) * base * 100) / 100,
    }));
  }, [benchmark, data, range]);
  const benchmarkStats = useMemo(() => calcStats(benchmarkData), [benchmarkData]);
  const relativeReturn = useMemo(() => {
    if (!stats || !benchmarkStats) return null;
    return stats.totalReturnPct - benchmarkStats.totalReturnPct;
  }, [stats, benchmarkStats]);

  const mergedData = useMemo(() => {
    const benchmarkByDate = new Map(benchmarkData.map((point) => [point.date, point.value]));
    return data.map((point) => ({
      ...point,
      benchmarkValue: benchmarkByDate.get(point.date) ?? null,
    }));
  }, [data, benchmarkData]);

  // Thin out data for 1Y/ALL to avoid too many ticks
  const chartData = useMemo(() => {
    if (mergedData.length <= 120) return mergedData;
    const step = Math.ceil(mergedData.length / 120);
    return mergedData.filter((_, i) => i % step === 0 || i === mergedData.length - 1);
  }, [mergedData]);

  const xTickCount = range === '1D' || range === '1W' ? undefined : 6;
  const assetOptions = useMemo(
    () => [
      { value: 'all', label: 'All Assets' },
      { value: 'stock', label: ASSET_TYPE_CONFIG.stock.label },
      { value: 'etf', label: ASSET_TYPE_CONFIG.etf.label },
      { value: 'cash', label: ASSET_TYPE_CONFIG.cash.label },
      { value: 'crypto', label: ASSET_TYPE_CONFIG.crypto.label },
    ],
    []
  );
  const benchmarkOptions = useMemo(
    () => [
      { value: 'none', label: 'No Benchmark' },
      ...BENCHMARK_SERIES.map((series) => ({ value: series.id, label: series.label })),
    ],
    []
  );

  if (!portfolio) {
    return <EmptyState message="No portfolio data available" />;
  }

  if (perfLoading) {
    return <EmptyState message="Loading performance history..." />;
  }

  if (perfIsEmpty) {
    return (
      <div style={{ ...PANEL }}>
        <EmptyState
          message="Performance history will appear here after your first price refresh."
          action={onRefresh ? { label: 'Refresh Prices', onClick: onRefresh } : undefined}
        />
      </div>
    );
  }

  if (filteredHoldings.length === 0) {
    return (
      <div style={{ ...PANEL }}>
        <div style={{ display: 'flex', gap: 12, marginBottom: 20, flexWrap: 'wrap' }}>
          <div style={{ width: 180 }}>
            <Select
              value={accountFilter}
              onChange={(value) => updateParam('account', value)}
              options={[
                { value: 'all', label: 'All Accounts' },
                ...ACCOUNT_OPTIONS.map((option) => ({ value: option.value, label: option.label })),
              ]}
            />
          </div>
          <div style={{ width: 160 }}>
            <Select
              value={assetFilter}
              onChange={(value) => updateParam('asset', value)}
              options={assetOptions}
            />
          </div>
        </div>
        <EmptyState message="No holdings match the selected account and asset filters" />
      </div>
    );
  }

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '1px',
        background: 'var(--border-primary)',
      }}
    >
      {/* Range selector + main chart */}
      <div style={PANEL}>
        {/* Range buttons */}
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            flexWrap: 'wrap',
            gap: 16,
            marginBottom: 20,
          }}
        >
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            <div style={{ width: 180 }}>
              <Select
                value={accountFilter}
                onChange={(value) => updateParam('account', value)}
                options={[
                  { value: 'all', label: 'All Accounts' },
                  ...ACCOUNT_OPTIONS.map((option) => ({
                    value: option.value,
                    label: option.label,
                  })),
                ]}
              />
            </div>
            <div style={{ width: 160 }}>
              <Select
                value={assetFilter}
                onChange={(value) => updateParam('asset', value)}
                options={assetOptions}
              />
            </div>
            <div style={{ width: 180 }}>
              <Select
                value={benchmarkId}
                onChange={(value) => updateParam('benchmark', value)}
                options={benchmarkOptions}
              />
            </div>
          </div>
          <div style={{ display: 'flex', gap: 1 }}>
            {RANGES.map((r) => (
              <button
                key={r}
                onClick={() => updateParam('range', r)}
                style={{
                  padding: '4px 12px',
                  fontFamily: 'var(--font-mono)',
                  fontSize: 11,
                  textTransform: 'uppercase',
                  letterSpacing: '0.06em',
                  cursor: 'pointer',
                  borderRadius: '2px',
                  background: range === r ? 'var(--color-accent)' : 'transparent',
                  color: range === r ? '#fff' : 'var(--text-secondary)',
                  border: range === r ? 'none' : '1px solid var(--border-primary)',
                }}
              >
                {r}
              </button>
            ))}
          </div>
        </div>

        {/* Area chart */}
        <ResponsiveContainer width="100%" height={280}>
          <AreaChart data={chartData} margin={{ top: 4, right: 0, left: 10, bottom: 0 }}>
            <defs>
              <linearGradient id="valueGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="var(--color-accent)" stopOpacity={0.3} />
                <stop offset="95%" stopColor="var(--color-accent)" stopOpacity={0} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="var(--border-subtle)" vertical={false} />
            <XAxis
              dataKey="date"
              tickFormatter={(v) => formatAxisDate(v, range)}
              tick={{ fill: 'var(--text-muted)', fontSize: 10, fontFamily: 'var(--font-mono)' }}
              axisLine={{ stroke: 'var(--border-primary)' }}
              tickLine={false}
              tickCount={xTickCount}
              interval="preserveStartEnd"
            />
            <YAxis
              tickFormatter={(v) => formatCompact(v)}
              tick={{ fill: 'var(--text-muted)', fontSize: 10, fontFamily: 'var(--font-mono)' }}
              axisLine={false}
              tickLine={false}
              width={62}
            />
            <Tooltip
              content={<CustomTooltip currency={baseCurrency} />}
              cursor={{ stroke: 'var(--text-muted)', strokeWidth: 1, strokeDasharray: '4 2' }}
            />
            <Area
              type="monotone"
              dataKey="value"
              stroke="var(--color-accent)"
              strokeWidth={2}
              fill="url(#valueGrad)"
              dot={false}
              activeDot={{
                r: 3,
                fill: 'var(--color-accent)',
                stroke: 'var(--bg-surface)',
                strokeWidth: 2,
              }}
            />
            {benchmark && benchmarkData.length > 0 && (
              <Area
                type="monotone"
                dataKey="benchmarkValue"
                stroke="var(--color-warning)"
                strokeWidth={2}
                fillOpacity={0}
                dot={false}
                isAnimationActive={false}
              />
            )}
          </AreaChart>
        </ResponsiveContainer>
      </div>

      {/* Daily returns bar chart */}
      <div style={{ ...PANEL, paddingTop: 14, paddingBottom: 14 }}>
        <div
          style={{
            fontSize: 10,
            color: 'var(--text-muted)',
            fontFamily: 'var(--font-mono)',
            textTransform: 'uppercase',
            letterSpacing: '0.08em',
            marginBottom: 8,
          }}
        >
          Daily Returns
        </div>
        <ResponsiveContainer width="100%" height={100}>
          <BarChart data={chartData} margin={{ top: 0, right: 0, left: 10, bottom: 0 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="var(--border-subtle)" vertical={false} />
            <XAxis dataKey="date" hide />
            <YAxis
              tickFormatter={(v) => `${v.toFixed(1)}%`}
              tick={{ fill: 'var(--text-muted)', fontSize: 9, fontFamily: 'var(--font-mono)' }}
              axisLine={false}
              tickLine={false}
              width={40}
            />
            <Tooltip
              contentStyle={{
                background: 'var(--bg-surface)',
                border: '1px solid var(--border-primary)',
                borderRadius: 0,
                fontSize: 11,
                fontFamily: 'var(--font-mono)',
              }}
              formatter={(v: unknown) => [`${Number(v).toFixed(2)}%`, 'Daily Return']}
              labelStyle={{ color: 'var(--text-secondary)' }}
            />
            <Bar dataKey="dailyReturn" maxBarSize={8}>
              {chartData.map((entry, i) => (
                <Cell
                  key={i}
                  fill={entry.dailyReturn >= 0 ? 'var(--color-gain)' : 'var(--color-loss)'}
                />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>

      {/* Stats row */}
      {stats && (
        <div
          style={{
            ...PANEL,
            display: 'grid',
            gridTemplateColumns: 'repeat(4, 1fr)',
            gap: 0,
          }}
        >
          {[
            {
              label: 'Total Return',
              value: `${stats.totalReturn >= 0 ? '+' : ''}${formatCurrency(stats.totalReturn, baseCurrency)}`,
              sub: formatPercent(stats.totalReturnPct),
              color: pnlColor(stats.totalReturn),
            },
            {
              label: 'Period High / Low',
              value: formatCompact(stats.periodHigh),
              sub: formatCompact(stats.periodLow),
              color: 'var(--text-primary)',
            },
            {
              label: 'Max Drawdown',
              value: `-${stats.maxDrawdown.toFixed(1)}%`,
              sub: 'peak to trough',
              color: 'var(--color-loss)',
            },
            {
              label: 'Annualized Volatility',
              value: `${stats.volatility.toFixed(1)}%`,
              sub: 'std dev of daily returns',
              color: 'var(--color-warning)',
            },
            {
              label: 'Best Day',
              value: formatPercent(stats.bestDay.pct),
              sub: stats.bestDay.date,
              color: 'var(--color-gain)',
            },
            {
              label: 'Worst Day',
              value: formatPercent(stats.worstDay.pct),
              sub: stats.worstDay.date,
              color: 'var(--color-loss)',
            },
            {
              label: 'Positions',
              value: `${filteredHoldings.length} holdings`,
              sub: `${accountFilter === 'all' ? 'all accounts' : accountFilter.toUpperCase()} · ${assetFilter === 'all' ? 'all assets' : assetFilter.toUpperCase()}`,
              color: 'var(--text-secondary)',
            },
            {
              label: 'Current Value',
              value: formatCurrency(data[data.length - 1]?.value ?? 0, baseCurrency),
              sub: `as of last data point · ${baseCurrency}`,
              color: 'var(--text-primary)',
            },
            {
              label: 'Benchmark',
              value: benchmark?.label ?? 'None',
              sub:
                relativeReturn === null
                  ? 'overlay disabled'
                  : `${relativeReturn >= 0 ? '+' : ''}${relativeReturn.toFixed(2)}% vs benchmark`,
              color: relativeReturn === null ? 'var(--text-secondary)' : pnlColor(relativeReturn),
            },
            {
              label: 'Benchmark Return',
              value:
                benchmarkStats && benchmark ? formatPercent(benchmarkStats.totalReturnPct) : '—',
              sub: benchmark ? `${benchmark.label} · ${range}` : 'select a benchmark',
              color:
                benchmarkStats && benchmark
                  ? pnlColor(benchmarkStats.totalReturnPct)
                  : 'var(--text-secondary)',
            },
          ].map((s, i) => (
            <div
              key={s.label}
              style={{
                padding: '14px 16px',
                borderRight: i % 4 < 3 ? '1px solid var(--border-subtle)' : 'none',
                borderBottom: i < 4 ? '1px solid var(--border-subtle)' : 'none',
              }}
            >
              <div style={STAT_LABEL}>{s.label}</div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 16,
                  fontWeight: 600,
                  color: s.color,
                }}
              >
                {s.value}
              </div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 11,
                  color: 'var(--text-muted)',
                  marginTop: 2,
                }}
              >
                {s.sub}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
