import { useMemo, useState } from 'react';
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
import { ALL_PERF_DATA, calcStats, filterByRange } from '../lib/perfMockData';
import { formatCurrency, formatCompact, formatPercent } from '../lib/format';
import { pnlColor } from '../lib/colors';

type Range = '1D' | '1W' | '1M' | '3M' | '6M' | '1Y' | 'ALL';
const RANGES: Range[] = ['1D', '1W', '1M', '3M', '6M', '1Y', 'ALL'];

const PANEL: React.CSSProperties = {
  background: 'var(--bg-surface)',
  border: '1px solid var(--border-primary)',
  padding: '20px',
};

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
}: {
  active?: boolean;
  payload?: Array<{ value: number; payload: { dailyReturn: number } }>;
  label?: string;
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
      <div style={{ color: 'var(--text-primary)', fontWeight: 600 }}>{formatCurrency(val)}</div>
      <div style={{ color: pnlColor(daily), marginTop: 2 }}>{formatPercent(daily)} daily</div>
    </div>
  );
}

export function Performance() {
  const [range, setRange] = useState<Range>('1Y');

  const data = useMemo(() => filterByRange(ALL_PERF_DATA, range), [range]);
  const stats = useMemo(() => calcStats(data), [data]);

  // Thin out data for 1Y/ALL to avoid too many ticks
  const chartData = useMemo(() => {
    if (data.length <= 120) return data;
    const step = Math.ceil(data.length / 120);
    return data.filter((_, i) => i % step === 0 || i === data.length - 1);
  }, [data]);

  const xTickCount = range === '1D' || range === '1W' ? undefined : 6;

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
        <div style={{ display: 'flex', gap: 1, marginBottom: 20 }}>
          {RANGES.map((r) => (
            <button
              key={r}
              onClick={() => setRange(r)}
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
              content={<CustomTooltip />}
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
              value: `${stats.totalReturn >= 0 ? '+' : ''}${formatCurrency(stats.totalReturn)}`,
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
              value: String(data.length) + ' days',
              sub: `${range} range`,
              color: 'var(--text-secondary)',
            },
            {
              label: 'Current Value',
              value: formatCurrency(data[data.length - 1]?.value ?? 0),
              sub: 'as of last data point',
              color: 'var(--text-primary)',
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
