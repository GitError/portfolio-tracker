import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Cell,
  ReferenceLine,
} from 'recharts';
import { usePortfolio } from '../hooks/usePortfolio';
import { useStressTest } from '../hooks/useStressTest';
import { createPresetScenarios, fxShockKey, ASSET_TYPE_CONFIG } from '../lib/constants';
import { formatCurrency, formatPercent, formatCompact } from '../lib/format';
import { pnlColor } from '../lib/colors';
import { EmptyState } from './ui/EmptyState';
import { Select } from './ui/Select';
import type { StressScenario } from '../types/portfolio';

// ─── Shock state keyed as the scenario.shocks keys ───────────────────────────
type ShockMap = Record<string, number>; // values are decimals e.g. -0.20

const ASSET_SLIDERS: { key: string; label: string }[] = [
  { key: 'stock', label: 'Stocks' },
  { key: 'etf', label: 'ETFs' },
  { key: 'crypto', label: 'Crypto' },
];

const ZERO_SHOCKS: ShockMap = {
  stock: 0,
  etf: 0,
  crypto: 0,
};

const PANEL: React.CSSProperties = {
  background: 'var(--bg-surface)',
  border: '1px solid var(--border-primary)',
  padding: '20px',
};

const SECTION_TITLE: React.CSSProperties = {
  fontSize: 10,
  color: 'var(--text-muted)',
  fontFamily: 'var(--font-mono)',
  textTransform: 'uppercase',
  letterSpacing: '0.1em',
  marginBottom: 12,
  paddingBottom: 6,
  borderBottom: '1px solid var(--border-subtle)',
};

const TD: React.CSSProperties = {
  padding: '6px 10px',
  fontSize: 11,
  borderBottom: '1px solid var(--border-subtle)',
  borderRight: '1px solid var(--border-subtle)',
  whiteSpace: 'nowrap',
};

// ─── Slider ───────────────────────────────────────────────────────────────────
function ShockSlider({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number;
  onChange: (v: number) => void;
}) {
  const pct = Math.round(value * 100);
  const color =
    value > 0 ? 'var(--color-gain)' : value < 0 ? 'var(--color-loss)' : 'var(--color-accent)';
  // Fill gradient position: 0 = -50%, 50 = 0%, 100 = +50%
  const pos = ((value * 100 + 50) / 100) * 100; // 0-100
  const gradStop =
    value < 0
      ? `var(--color-loss) 0%, var(--color-loss) ${pos}%, var(--border-primary) ${pos}%`
      : value > 0
        ? `var(--border-primary) 0%, var(--border-primary) ${pos}%, var(--color-gain) ${pos}%`
        : `var(--border-primary) 0%`;

  return (
    <div style={{ marginBottom: 14 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 5 }}>
        <span
          style={{ fontSize: 12, color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)' }}
        >
          {label}
        </span>
        <span
          style={{
            fontSize: 12,
            fontFamily: 'var(--font-mono)',
            fontWeight: 600,
            color,
            minWidth: 50,
            textAlign: 'right',
          }}
        >
          {pct >= 0 ? '+' : ''}
          {pct}%
        </span>
      </div>
      <input
        type="range"
        min={-50}
        max={50}
        step={1}
        value={pct}
        onChange={(e) => onChange(parseInt(e.target.value) / 100)}
        style={{
          width: '100%',
          height: 4,
          appearance: 'none',
          WebkitAppearance: 'none',
          background: `linear-gradient(to right, ${gradStop})`,
          outline: 'none',
          cursor: 'pointer',
          borderRadius: 0,
        }}
      />
    </div>
  );
}

// ─── Comparison chart ─────────────────────────────────────────────────────────
function ComparisonChart({
  totalValue,
  currency,
  scenarios,
}: {
  totalValue: number;
  currency: string;
  scenarios: StressScenario[];
}) {
  const data = scenarios.map((s) => {
    let stressed = 0;
    // crude estimate: no holdings breakdown, use asset shock weighted avg
    const avg =
      Object.values(s.shocks).reduce((a, b) => a + b, 0) /
      Math.max(Object.keys(s.shocks).length, 1);
    stressed = totalValue * (1 + avg);
    const impact = stressed - totalValue;
    return { name: s.name, impact: Math.round(impact), pct: (impact / totalValue) * 100 };
  });

  return (
    <div style={{ ...PANEL, marginTop: 1 }}>
      <div style={SECTION_TITLE}>Scenario Comparison</div>
      <ResponsiveContainer width="100%" height={140}>
        <BarChart data={data} margin={{ top: 0, right: 0, left: 10, bottom: 0 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="var(--border-subtle)" vertical={false} />
          <XAxis
            dataKey="name"
            tick={{ fill: 'var(--text-muted)', fontSize: 9, fontFamily: 'var(--font-mono)' }}
            axisLine={{ stroke: 'var(--border-primary)' }}
            tickLine={false}
          />
          <YAxis
            tickFormatter={(v) => formatCompact(v)}
            tick={{ fill: 'var(--text-muted)', fontSize: 9, fontFamily: 'var(--font-mono)' }}
            axisLine={false}
            tickLine={false}
            width={58}
          />
          <ReferenceLine y={0} stroke="var(--border-primary)" />
          <Tooltip
            contentStyle={{
              background: 'var(--bg-surface)',
              border: '1px solid var(--border-primary)',
              borderRadius: 0,
              fontSize: 11,
              fontFamily: 'var(--font-mono)',
            }}
            formatter={(v: unknown) => [formatCurrency(Number(v), currency), 'Impact']}
            labelStyle={{ color: 'var(--text-secondary)' }}
          />
          <Bar dataKey="impact" maxBarSize={40}>
            {data.map((d, i) => (
              <Cell key={i} fill={d.impact >= 0 ? 'var(--color-gain)' : 'var(--color-loss)'} />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

// ─── Main component ───────────────────────────────────────────────────────────
export function StressTest() {
  const { portfolio, holdings } = usePortfolio();
  const { result, loading, runTest } = useStressTest();
  const baseCurrency = portfolio?.baseCurrency ?? 'CAD';
  const presetScenarios = useMemo(() => createPresetScenarios(baseCurrency), [baseCurrency]);
  const presetNames = useMemo(
    () => [...presetScenarios.map((s) => s.name), 'Custom'],
    [presetScenarios]
  );
  const [presetName, setPresetName] = useState<string>('Mild Correction');
  const [shocks, setShocks] = useState<ShockMap>(ZERO_SHOCKS);
  const [showComparison, setShowComparison] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Detect which FX sliders are relevant to held currencies
  const activeFxSliders = useMemo(() => {
    const currencies = new Set((portfolio?.holdings ?? []).map((h) => h.currency.toUpperCase()));
    return [...currencies]
      .filter((currency) => currency.toUpperCase() !== baseCurrency.toUpperCase())
      .sort()
      .map((currency) => ({
        key: fxShockKey(currency, baseCurrency),
        label: `${currency}/${baseCurrency}`,
      }));
  }, [portfolio, baseCurrency]);

  useEffect(() => {
    if (presetName === 'Custom') return;
    const preset =
      presetScenarios.find((scenario) => scenario.name === presetName) ?? presetScenarios[0];
    setShocks({ ...ZERO_SHOCKS, ...preset.shocks });
  }, [presetName, presetScenarios]);

  // Run stress test whenever shocks change (debounced 150ms)
  const scheduleRun = useCallback(
    (nextShocks: ShockMap) => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => {
        const scenario: StressScenario = {
          name: presetName,
          shocks: Object.fromEntries(Object.entries(nextShocks).filter(([, v]) => v !== 0)),
        };
        runTest(scenario, portfolio);
      }, 150);
    },
    [portfolio, presetName, runTest]
  );

  // Initial run on mount / portfolio load
  useEffect(() => {
    if (portfolio) scheduleRun(shocks);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [portfolio]);

  function handlePresetChange(name: string) {
    setPresetName(name);
    const preset = presetScenarios.find((s) => s.name === name);
    const next: ShockMap = { ...ZERO_SHOCKS, ...(preset?.shocks ?? {}) };
    setShocks(next);
    scheduleRun(next);
  }

  function handleSliderChange(key: string, value: number) {
    setPresetName('Custom');
    const next = { ...shocks, [key]: value };
    setShocks(next);
    scheduleRun(next);
  }

  const allZero = Object.values(shocks).every((v) => v === 0);

  if (holdings.length === 0) {
    return <EmptyState message="Add holdings to run stress tests" />;
  }

  const waterfallData = result
    ? [...result.holdingBreakdown].sort((a, b) => a.impact - b.impact).filter((h) => h.impact !== 0)
    : [];

  return (
    <div>
      {/* Compare toggle */}
      <div style={{ display: 'flex', justifyContent: 'flex-end', marginBottom: 12 }}>
        <button
          onClick={() => setShowComparison((v) => !v)}
          style={{
            padding: '5px 14px',
            background: showComparison ? 'var(--color-accent)' : 'transparent',
            border: '1px solid var(--border-primary)',
            color: showComparison ? '#fff' : 'var(--text-secondary)',
            borderRadius: '2px',
            fontFamily: 'var(--font-mono)',
            fontSize: 11,
            textTransform: 'uppercase',
            letterSpacing: '0.06em',
            cursor: 'pointer',
          }}
        >
          Compare Scenarios
        </button>
      </div>

      {showComparison && (
        <ComparisonChart
          totalValue={portfolio?.totalValue ?? 0}
          currency={baseCurrency}
          scenarios={presetScenarios}
        />
      )}

      {/* Main two-column layout */}
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: '35% 65%',
          gap: 1,
          background: 'var(--border-primary)',
        }}
      >
        {/* ─── LEFT: Scenario controls ─── */}
        <div
          style={{
            ...PANEL,
            display: 'flex',
            flexDirection: 'column',
            gap: 0,
            background: 'var(--bg-surface)',
          }}
        >
          {/* Preset selector */}
          <div style={{ marginBottom: 20 }}>
            <div style={SECTION_TITLE}>Preset Scenario</div>
            <Select
              value={presetName}
              onChange={handlePresetChange}
              options={presetNames.map((n) => ({ value: n, label: n }))}
            />
          </div>

          {/* Asset class shocks */}
          <div style={{ marginBottom: 20 }}>
            <div style={SECTION_TITLE}>Asset Class Shocks</div>
            {ASSET_SLIDERS.map(({ key, label }) => (
              <ShockSlider
                key={key}
                label={label}
                value={shocks[key] ?? 0}
                onChange={(v) => handleSliderChange(key, v)}
              />
            ))}
          </div>

          {/* FX shocks */}
          {activeFxSliders.length > 0 && (
            <div>
              <div style={SECTION_TITLE}>Currency Shocks</div>
              <div
                style={{
                  fontSize: 10,
                  color: 'var(--text-muted)',
                  fontFamily: 'var(--font-mono)',
                  marginBottom: 10,
                }}
              >
                Positive = {baseCurrency} weakens (foreign assets worth more in {baseCurrency})
              </div>
              {activeFxSliders.map(({ key, label }) => (
                <ShockSlider
                  key={key}
                  label={label}
                  value={shocks[key] ?? 0}
                  onChange={(v) => handleSliderChange(key, v)}
                />
              ))}
            </div>
          )}

          {loading && (
            <div
              style={{
                fontSize: 10,
                color: 'var(--text-muted)',
                fontFamily: 'var(--font-mono)',
                textAlign: 'center',
                marginTop: 8,
              }}
            >
              Calculating...
            </div>
          )}
        </div>

        {/* ─── RIGHT: Results ─── */}
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 1,
            background: 'var(--border-primary)',
          }}
        >
          {/* Summary card */}
          <div
            style={{
              ...PANEL,
              background:
                result && result.totalImpact < -1
                  ? 'linear-gradient(135deg, #1a0a0e 0%, var(--bg-surface) 60%)'
                  : result && result.totalImpact > 1
                    ? 'linear-gradient(135deg, #0a1a12 0%, var(--bg-surface) 60%)'
                    : 'var(--bg-surface)',
            }}
          >
            {allZero || !result ? (
              <div style={{ textAlign: 'center', padding: '24px 0' }}>
                <div
                  style={{
                    fontFamily: 'var(--font-mono)',
                    color: 'var(--text-muted)',
                    fontSize: 12,
                    letterSpacing: '0.1em',
                  }}
                >
                  NO SCENARIO APPLIED
                </div>
                <div
                  style={{
                    fontFamily: 'var(--font-mono)',
                    color: 'var(--text-muted)',
                    fontSize: 11,
                    marginTop: 6,
                  }}
                >
                  Adjust sliders to see impact
                </div>
              </div>
            ) : (
              <>
                <div style={{ display: 'flex', alignItems: 'center', gap: 20, marginBottom: 16 }}>
                  <div>
                    <div
                      style={{
                        fontSize: 10,
                        color: 'var(--text-muted)',
                        fontFamily: 'var(--font-mono)',
                        textTransform: 'uppercase',
                        letterSpacing: '0.08em',
                        marginBottom: 4,
                      }}
                    >
                      Current ({baseCurrency})
                    </div>
                    <div
                      style={{
                        fontFamily: 'var(--font-mono)',
                        fontSize: 22,
                        fontWeight: 700,
                        color: 'var(--text-primary)',
                      }}
                    >
                      {formatCurrency(result.currentValue, baseCurrency)}
                    </div>
                  </div>
                  <div style={{ fontSize: 18, color: 'var(--text-muted)' }}>→</div>
                  <div>
                    <div
                      style={{
                        fontSize: 10,
                        color: 'var(--text-muted)',
                        fontFamily: 'var(--font-mono)',
                        textTransform: 'uppercase',
                        letterSpacing: '0.08em',
                        marginBottom: 4,
                      }}
                    >
                      Stressed ({baseCurrency})
                    </div>
                    <div
                      style={{
                        fontFamily: 'var(--font-mono)',
                        fontSize: 22,
                        fontWeight: 700,
                        color: pnlColor(result.totalImpact),
                      }}
                    >
                      {formatCurrency(result.stressedValue, baseCurrency)}
                    </div>
                  </div>
                </div>
                <div style={{ borderTop: '1px solid var(--border-subtle)', paddingTop: 12 }}>
                  <span
                    style={{
                      fontFamily: 'var(--font-mono)',
                      fontSize: 28,
                      fontWeight: 700,
                      color: pnlColor(result.totalImpact),
                    }}
                  >
                    {result.totalImpact >= 0 ? '+' : ''}
                    {formatCurrency(result.totalImpact, baseCurrency)}
                  </span>
                  <span
                    style={{
                      fontFamily: 'var(--font-mono)',
                      fontSize: 16,
                      color: pnlColor(result.totalImpact),
                      marginLeft: 12,
                    }}
                  >
                    ({formatPercent(result.totalImpactPercent)})
                  </span>
                </div>
              </>
            )}
          </div>

          {/* Waterfall chart */}
          {result && !allZero && waterfallData.length > 0 && (
            <div style={PANEL}>
              <div style={SECTION_TITLE}>Impact by Holding</div>
              <ResponsiveContainer width="100%" height={Math.max(180, waterfallData.length * 28)}>
                <BarChart
                  layout="vertical"
                  data={waterfallData}
                  margin={{ top: 0, right: 10, left: 0, bottom: 0 }}
                >
                  <CartesianGrid
                    strokeDasharray="3 3"
                    stroke="var(--border-subtle)"
                    horizontal={false}
                  />
                  <XAxis
                    type="number"
                    tickFormatter={(v) => formatCompact(v)}
                    tick={{
                      fill: 'var(--text-muted)',
                      fontSize: 9,
                      fontFamily: 'var(--font-mono)',
                    }}
                    axisLine={{ stroke: 'var(--border-primary)' }}
                    tickLine={false}
                  />
                  <YAxis
                    type="category"
                    dataKey="symbol"
                    tick={{
                      fill: 'var(--text-secondary)',
                      fontSize: 11,
                      fontFamily: 'var(--font-mono)',
                      fontWeight: 600,
                    }}
                    axisLine={false}
                    tickLine={false}
                    width={72}
                  />
                  <ReferenceLine x={0} stroke="var(--border-primary)" />
                  <Tooltip
                    contentStyle={{
                      background: 'var(--bg-surface)',
                      border: '1px solid var(--border-primary)',
                      borderRadius: 0,
                      fontSize: 11,
                      fontFamily: 'var(--font-mono)',
                    }}
                    content={({ active, payload }) => {
                      if (!active || !payload?.length) return null;
                      const d = payload[0].payload;
                      return (
                        <div
                          style={{
                            background: 'var(--bg-surface)',
                            border: '1px solid var(--border-primary)',
                            padding: '10px 14px',
                            fontSize: 11,
                            fontFamily: 'var(--font-mono)',
                          }}
                        >
                          <div
                            style={{
                              color: 'var(--text-primary)',
                              fontWeight: 700,
                              marginBottom: 4,
                            }}
                          >
                            {d.symbol} — {d.name}
                          </div>
                          <div style={{ color: 'var(--text-secondary)' }}>
                            Current: {formatCurrency(d.currentValue, baseCurrency)}
                          </div>
                          <div style={{ color: pnlColor(d.stressedValue - d.currentValue) }}>
                            Stressed: {formatCurrency(d.stressedValue, baseCurrency)}
                          </div>
                          <div style={{ color: 'var(--text-muted)' }}>
                            Shock: {formatPercent(d.shockApplied * 100)}
                          </div>
                          <div style={{ color: pnlColor(d.impact), fontWeight: 600, marginTop: 4 }}>
                            Impact: {d.impact >= 0 ? '+' : ''}
                            {formatCurrency(d.impact, baseCurrency)}
                          </div>
                        </div>
                      );
                    }}
                  />
                  <Bar dataKey="impact" maxBarSize={16}>
                    {waterfallData.map((d, i) => (
                      <Cell
                        key={i}
                        fill={d.impact >= 0 ? 'var(--color-gain)' : 'var(--color-loss)'}
                      />
                    ))}
                  </Bar>
                </BarChart>
              </ResponsiveContainer>
            </div>
          )}

          {/* Breakdown table */}
          {result && !allZero && (
            <div style={{ ...PANEL, overflow: 'auto', maxHeight: 360 }}>
              <div style={SECTION_TITLE}>Breakdown</div>
              <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 11 }}>
                <thead style={{ position: 'sticky', top: 0 }}>
                  <tr>
                    {[
                      'Symbol',
                      'Type',
                      `Current Value (${baseCurrency})`,
                      'Shock',
                      `Stressed Value (${baseCurrency})`,
                      `Impact (${baseCurrency})`,
                      'Impact (%)',
                    ].map((col) => (
                      <th
                        key={col}
                        style={{
                          ...TD,
                          background: 'var(--bg-surface-alt)',
                          color: 'var(--text-secondary)',
                          fontWeight: 600,
                          textTransform: 'uppercase',
                          letterSpacing: '0.06em',
                          fontSize: 9,
                          textAlign: col === 'Symbol' || col === 'Type' ? 'left' : 'right',
                          borderBottom: '1px solid var(--border-primary)',
                        }}
                      >
                        {col}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {[...result.holdingBreakdown]
                    .sort((a, b) => a.impact - b.impact)
                    .map((h, i) => {
                      const holding = portfolio?.holdings.find((p) => p.id === h.holdingId);
                      const bg = i % 2 === 0 ? 'var(--bg-surface)' : 'var(--bg-surface-alt)';
                      return (
                        <tr key={h.holdingId} style={{ background: bg }}>
                          <td
                            style={{
                              ...TD,
                              fontFamily: 'var(--font-mono)',
                              fontWeight: 700,
                              color: 'var(--text-primary)',
                            }}
                          >
                            {h.symbol}
                          </td>
                          <td style={{ ...TD, color: 'var(--text-secondary)' }}>
                            {holding
                              ? (ASSET_TYPE_CONFIG[holding.assetType]?.label ?? holding.assetType)
                              : '—'}
                          </td>
                          <td
                            style={{
                              ...TD,
                              textAlign: 'right',
                              fontFamily: 'var(--font-mono)',
                              color: 'var(--text-secondary)',
                            }}
                          >
                            {formatCurrency(h.currentValue, baseCurrency)}
                          </td>
                          <td
                            style={{
                              ...TD,
                              textAlign: 'right',
                              fontFamily: 'var(--font-mono)',
                              color: pnlColor(h.shockApplied),
                            }}
                          >
                            {h.shockApplied !== 0 ? formatPercent(h.shockApplied * 100) : '—'}
                          </td>
                          <td
                            style={{
                              ...TD,
                              textAlign: 'right',
                              fontFamily: 'var(--font-mono)',
                              color: pnlColor(h.impact),
                            }}
                          >
                            {formatCurrency(h.stressedValue, baseCurrency)}
                          </td>
                          <td
                            style={{
                              ...TD,
                              textAlign: 'right',
                              fontFamily: 'var(--font-mono)',
                              fontWeight: 600,
                              color: pnlColor(h.impact),
                            }}
                          >
                            {h.impact !== 0
                              ? `${h.impact >= 0 ? '+' : ''}${formatCurrency(h.impact, baseCurrency)}`
                              : '—'}
                          </td>
                          <td
                            style={{
                              ...TD,
                              textAlign: 'right',
                              fontFamily: 'var(--font-mono)',
                              color: pnlColor(h.impact),
                              borderRight: 'none',
                            }}
                          >
                            {h.impact !== 0
                              ? formatPercent((h.impact / h.currentValue) * 100)
                              : '—'}
                          </td>
                        </tr>
                      );
                    })}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </div>

      {/* Slider thumb global style injection */}
      <style>{`
        input[type='range']::-webkit-slider-thumb {
          -webkit-appearance: none;
          appearance: none;
          width: 12px;
          height: 12px;
          background: var(--color-accent);
          border-radius: 0;
          cursor: pointer;
        }
        input[type='range']::-moz-range-thumb {
          width: 12px;
          height: 12px;
          background: var(--color-accent);
          border-radius: 0;
          border: none;
          cursor: pointer;
        }
      `}</style>
    </div>
  );
}
