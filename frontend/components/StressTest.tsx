import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
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
import { HelpCircle } from 'lucide-react';
import { usePortfolio } from '../hooks/usePortfolio';
import { useStressTest } from '../hooks/useStressTest';
import {
  createPresetScenarioInfo,
  createPresetScenarios,
  fxShockKey,
  ASSET_TYPE_CONFIG,
  ACCOUNT_OPTIONS,
} from '../lib/constants';
import { formatCurrency, formatPercent, formatCompact } from '../lib/format';
import { pnlColor } from '../lib/colors';
import { EmptyState } from './ui/EmptyState';
import { Select } from './ui/Select';
import { CollapsiblePanel } from './ui/CollapsiblePanel';
import { StressTestInfo } from './StressTestInfo';
import { config } from '../lib/config';
import type {
  AccountType,
  AssetType,
  PortfolioSnapshot,
  StressScenario,
  StressScenarioInfo,
} from '../types/portfolio';

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

const ASSET_FILTER_OPTIONS: { value: string; label: string }[] = [
  { value: 'all', label: 'All Assets' },
  { value: 'stock', label: 'Stocks' },
  { value: 'etf', label: 'ETFs' },
  { value: 'crypto', label: 'Crypto' },
  { value: 'cash', label: 'Cash' },
];

const ACCOUNT_FILTER_OPTIONS: { value: string; label: string }[] = [
  { value: 'all', label: 'All Accounts' },
  ...ACCOUNT_OPTIONS,
];

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

// ─── Compute stressed value for a scenario inline ─────────────────────────────
function computeScenarioLocally(
  snapshot: PortfolioSnapshot,
  scenario: StressScenario
): {
  stressedValue: number;
  totalImpact: number;
  totalImpactPercent: number;
  topImpacted: { symbol: string; impact: number }[];
} {
  let totalStressed = 0;
  const breakdown: { symbol: string; impact: number }[] = [];

  for (const h of snapshot.holdings) {
    const assetShock = scenario.shocks[h.assetType] ?? 0;
    const fxKey = fxShockKey(h.currency, snapshot.baseCurrency);
    const fxShock =
      h.currency.toUpperCase() === snapshot.baseCurrency.toUpperCase()
        ? 0
        : (scenario.shocks[fxKey] ?? 0);

    const currentValue = h.marketValueCad;
    const stressedValue = currentValue * (1 + assetShock) * (1 + fxShock);
    const impact = stressedValue - currentValue;
    totalStressed += stressedValue;
    breakdown.push({ symbol: h.symbol, impact });
  }

  const currentValue = snapshot.totalValue;
  const totalImpact = totalStressed - currentValue;
  const totalImpactPercent = currentValue !== 0 ? (totalImpact / currentValue) * 100 : 0;

  const topImpacted = [...breakdown]
    .sort((a, b) => Math.abs(b.impact) - Math.abs(a.impact))
    .slice(0, 2);

  return { stressedValue: totalStressed, totalImpact, totalImpactPercent, topImpacted };
}

// ─── Scenario comparison table ────────────────────────────────────────────────
function ScenarioComparison({
  portfolio,
  scenarios,
}: {
  portfolio: PortfolioSnapshot;
  scenarios: StressScenarioInfo[];
}) {
  const baseCurrency = portfolio.baseCurrency;

  const rows = useMemo(
    () =>
      scenarios.map((s) => {
        const result = computeScenarioLocally(portfolio, s);
        return {
          name: s.name,
          description: s.description,
          stressedValue: result.stressedValue,
          totalImpact: result.totalImpact,
          totalImpactPercent: result.totalImpactPercent,
          topImpacted: result.topImpacted,
        };
      }),
    [portfolio, scenarios]
  );

  return (
    <div style={{ ...PANEL, marginBottom: 1 }}>
      <div style={SECTION_TITLE}>Scenario Comparison</div>
      <div style={{ overflowX: 'auto' }}>
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 11 }}>
          <thead>
            <tr>
              {[
                'Scenario',
                `Stressed Value (${baseCurrency})`,
                `Impact (${baseCurrency})`,
                'Impact (%)',
                'Top Impacted Holdings',
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
                    textAlign:
                      col === 'Scenario' || col === 'Top Impacted Holdings' ? 'left' : 'right',
                    borderBottom: '1px solid var(--border-primary)',
                  }}
                >
                  {col}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {/* Current value reference row */}
            <tr style={{ background: 'var(--bg-surface-alt)' }}>
              <td
                style={{
                  ...TD,
                  fontFamily: 'var(--font-mono)',
                  fontWeight: 700,
                  color: 'var(--text-primary)',
                }}
              >
                Current Value
              </td>
              <td
                style={{
                  ...TD,
                  textAlign: 'right',
                  fontFamily: 'var(--font-mono)',
                  fontWeight: 700,
                  color: 'var(--text-primary)',
                }}
              >
                {formatCurrency(portfolio.totalValue, baseCurrency)}
              </td>
              <td
                style={{
                  ...TD,
                  textAlign: 'right',
                  color: 'var(--text-muted)',
                  fontFamily: 'var(--font-mono)',
                }}
              >
                —
              </td>
              <td
                style={{
                  ...TD,
                  textAlign: 'right',
                  color: 'var(--text-muted)',
                  fontFamily: 'var(--font-mono)',
                }}
              >
                —
              </td>
              <td style={{ ...TD, color: 'var(--text-muted)', borderRight: 'none' }}>—</td>
            </tr>

            {/* Scenario rows */}
            {rows.map((row, i) => (
              <tr
                key={row.name}
                style={{ background: i % 2 === 0 ? 'var(--bg-surface)' : 'var(--bg-surface-alt)' }}
              >
                <td style={{ ...TD, fontFamily: 'var(--font-mono)' }}>
                  <div style={{ fontWeight: 600, color: 'var(--text-primary)' }}>{row.name}</div>
                  <div
                    style={{
                      fontSize: 10,
                      color: 'var(--text-muted)',
                      marginTop: 2,
                      fontFamily: 'var(--font-sans)',
                      whiteSpace: 'normal',
                      maxWidth: 220,
                    }}
                  >
                    {row.description}
                  </div>
                </td>
                <td
                  style={{
                    ...TD,
                    textAlign: 'right',
                    fontFamily: 'var(--font-mono)',
                    color: pnlColor(row.totalImpact),
                  }}
                >
                  {formatCurrency(row.stressedValue, baseCurrency)}
                </td>
                <td
                  style={{
                    ...TD,
                    textAlign: 'right',
                    fontFamily: 'var(--font-mono)',
                    fontWeight: 600,
                    color: pnlColor(row.totalImpact),
                  }}
                >
                  {row.totalImpact >= 0 ? '+' : ''}
                  {formatCurrency(row.totalImpact, baseCurrency)}
                </td>
                <td
                  style={{
                    ...TD,
                    textAlign: 'right',
                    fontFamily: 'var(--font-mono)',
                    fontWeight: 600,
                    color: pnlColor(row.totalImpact),
                  }}
                >
                  {formatPercent(row.totalImpactPercent)}
                </td>
                <td style={{ ...TD, borderRight: 'none' }}>
                  {row.topImpacted.length === 0 ? (
                    <span style={{ color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
                      —
                    </span>
                  ) : (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                      {row.topImpacted.map((h) => (
                        <div
                          key={h.symbol}
                          style={{
                            fontFamily: 'var(--font-mono)',
                            fontSize: 10,
                            color: pnlColor(h.impact),
                          }}
                        >
                          <span style={{ fontWeight: 700 }}>{h.symbol}</span>{' '}
                          <span>
                            {h.impact >= 0 ? '+' : ''}
                            {formatCompact(h.impact)}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ─── Resilience summary ───────────────────────────────────────────────────────
function ResilienceSummary({ portfolio }: { portfolio: PortfolioSnapshot | null }) {
  if (!portfolio || portfolio.holdings.length === 0) return null;

  const baseCurrency = portfolio.baseCurrency;

  // Largest single-holding risk by weight
  const largestHolding = [...portfolio.holdings].sort((a, b) => b.weight - a.weight)[0];

  // Max hit: holding with highest market value (worst case if it goes to zero)
  const maxHitHolding = [...portfolio.holdings].sort(
    (a, b) => b.marketValueCad - a.marketValueCad
  )[0];

  // Diversification score: 1 - HHI, normalized
  // Only non-cash holdings for diversification computation
  const nonCash = portfolio.holdings.filter((h) => h.assetType !== 'cash');
  const n = nonCash.length;
  let diversificationScore = 0;
  if (n > 1) {
    const totalNonCash = nonCash.reduce((sum, h) => sum + h.marketValueCad, 0);
    if (totalNonCash > 0) {
      const hhi = nonCash.reduce((sum, h) => {
        const w = h.marketValueCad / totalNonCash;
        return sum + w * w;
      }, 0);
      diversificationScore = ((1 - hhi) / (1 - 1 / n)) * 100;
    }
  } else if (n === 1) {
    diversificationScore = 0;
  }

  // FX exposure: % of portfolio in non-base-currency holdings
  const fxExposureValue = portfolio.holdings
    .filter((h) => h.currency.toUpperCase() !== baseCurrency.toUpperCase())
    .reduce((sum, h) => sum + h.marketValueCad, 0);
  const fxExposurePct =
    portfolio.totalValue > 0 ? (fxExposureValue / portfolio.totalValue) * 100 : 0;

  // Cash buffer: % of portfolio in cash positions
  const cashValue = portfolio.holdings
    .filter((h) => h.assetType === 'cash')
    .reduce((sum, h) => sum + h.marketValueCad, 0);
  const cashPct = portfolio.totalValue > 0 ? (cashValue / portfolio.totalValue) * 100 : 0;

  const STAT_CARD: React.CSSProperties = {
    background: 'var(--bg-surface)',
    border: '1px solid var(--border-primary)',
    padding: '14px 16px',
    flex: '1 1 0',
    minWidth: 160,
  };

  const STAT_LABEL: React.CSSProperties = {
    fontSize: 10,
    color: 'var(--text-muted)',
    fontFamily: 'var(--font-mono)',
    textTransform: 'uppercase',
    letterSpacing: '0.08em',
    marginBottom: 6,
  };

  const STAT_VALUE: React.CSSProperties = {
    fontSize: 20,
    fontFamily: 'var(--font-mono)',
    fontWeight: 700,
    color: 'var(--text-primary)',
    lineHeight: 1.2,
  };

  const STAT_SUB: React.CSSProperties = {
    fontSize: 11,
    fontFamily: 'var(--font-mono)',
    color: 'var(--text-secondary)',
    marginTop: 4,
  };

  return (
    <div style={{ marginTop: 1 }}>
      <div
        style={{
          ...PANEL,
          paddingBottom: 16,
          background: 'var(--bg-surface)',
        }}
      >
        <div style={SECTION_TITLE}>Portfolio Resilience</div>
        <div
          style={{ display: 'flex', gap: 1, flexWrap: 'wrap', background: 'var(--border-primary)' }}
        >
          {/* Largest single-holding risk */}
          <div style={STAT_CARD}>
            <div style={STAT_LABEL}>Largest Position</div>
            <div style={STAT_VALUE}>
              {largestHolding ? `${(largestHolding.weight * 100).toFixed(1)}%` : '—'}
            </div>
            <div style={STAT_SUB}>{largestHolding ? largestHolding.symbol : '—'} of portfolio</div>
          </div>

          {/* Max hit from one holding */}
          <div style={STAT_CARD}>
            <div style={STAT_LABEL}>Max Single-Holding Loss</div>
            <div style={{ ...STAT_VALUE, color: 'var(--color-loss)' }}>
              {maxHitHolding ? formatCompact(maxHitHolding.marketValueCad) : '—'}
            </div>
            <div style={STAT_SUB}>{maxHitHolding ? `${maxHitHolding.symbol} at zero` : '—'}</div>
          </div>

          {/* Diversification score */}
          <div style={STAT_CARD}>
            <div style={STAT_LABEL}>Diversification Score</div>
            <div
              style={{
                ...STAT_VALUE,
                color:
                  diversificationScore >= 70
                    ? 'var(--color-gain)'
                    : diversificationScore >= 40
                      ? 'var(--color-warning)'
                      : 'var(--color-loss)',
              }}
            >
              {n > 0 ? `${diversificationScore.toFixed(0)} / 100` : '—'}
            </div>
            <div style={STAT_SUB}>
              {n > 0
                ? diversificationScore >= 70
                  ? 'Well diversified'
                  : diversificationScore >= 40
                    ? 'Moderate concentration'
                    : 'Highly concentrated'
                : 'No non-cash holdings'}
            </div>
          </div>

          {/* FX exposure */}
          <div style={STAT_CARD}>
            <div style={STAT_LABEL}>Foreign Currency Exposure</div>
            <div style={STAT_VALUE}>{fxExposurePct.toFixed(1)}%</div>
            <div style={STAT_SUB}>
              {formatCompact(fxExposureValue)} in non-{baseCurrency}
            </div>
          </div>

          {/* Cash buffer */}
          <div style={STAT_CARD}>
            <div style={STAT_LABEL}>Cash Buffer</div>
            <div
              style={{
                ...STAT_VALUE,
                color:
                  cashPct >= 5
                    ? 'var(--color-gain)'
                    : cashPct > 0
                      ? 'var(--color-warning)'
                      : 'var(--text-secondary)',
              }}
            >
              {cashPct.toFixed(1)}%
            </div>
            <div style={STAT_SUB}>{formatCompact(cashValue)} in cash positions</div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ─── Filter helper ────────────────────────────────────────────────────────────
function applyHoldingFilters(
  snapshot: PortfolioSnapshot,
  accountFilter: 'all' | AccountType,
  assetFilter: 'all' | AssetType
): PortfolioSnapshot {
  if (accountFilter === 'all' && assetFilter === 'all') return snapshot;

  const filteredHoldings = snapshot.holdings.filter((h) => {
    const accountMatch = accountFilter === 'all' || h.account === accountFilter;
    const assetMatch = assetFilter === 'all' || h.assetType === assetFilter;
    return accountMatch && assetMatch;
  });

  const totalValue = filteredHoldings.reduce((sum, h) => sum + h.marketValueCad, 0);
  const totalCost = filteredHoldings.reduce((sum, h) => sum + h.costValueCad, 0);
  const totalGainLoss = totalValue - totalCost;
  const totalGainLossPercent = totalCost !== 0 ? (totalGainLoss / totalCost) * 100 : 0;
  const dailyPnl = filteredHoldings.reduce(
    (sum, h) => sum + h.marketValueCad * (h.dailyChangePercent / 100),
    0
  );

  // Re-compute weights relative to the filtered subset
  const holdingsWithWeight = filteredHoldings.map((h) => ({
    ...h,
    weight: totalValue !== 0 ? h.marketValueCad / totalValue : 0,
  }));

  return {
    ...snapshot,
    holdings: holdingsWithWeight,
    totalValue,
    totalCost,
    totalGainLoss,
    totalGainLossPercent,
    dailyPnl,
  };
}

// ─── Main component ───────────────────────────────────────────────────────────
export function StressTest() {
  const { portfolio, holdings } = usePortfolio();
  const { result, loading, runTest } = useStressTest();
  const [searchParams, setSearchParams] = useSearchParams();
  const baseCurrency = portfolio?.baseCurrency ?? 'CAD';
  const presetScenarioInfo = useMemo(() => createPresetScenarioInfo(baseCurrency), [baseCurrency]);
  const presetScenarios = useMemo(() => createPresetScenarios(baseCurrency), [baseCurrency]);
  const presetNames = useMemo(
    () => [...presetScenarios.map((s) => s.name), 'Custom'],
    [presetScenarios]
  );
  const [presetName, setPresetName] = useState<string>('Mild Correction');
  const [shocks, setShocks] = useState<ShockMap>(ZERO_SHOCKS);
  const [showComparison, setShowComparison] = useState(false);
  const [infoOpen, setInfoOpen] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const activePresetInfo = useMemo(
    () => presetScenarioInfo.find((scenario) => scenario.name === presetName) ?? null,
    [presetName, presetScenarioInfo]
  );

  // ─── Filter state persisted in URL ─────────────────────────────────────────
  const accountFilter = (searchParams.get('stressAccount') ?? 'all') as 'all' | AccountType;
  const assetFilter = (searchParams.get('stressAsset') ?? 'all') as 'all' | AssetType;

  function setAccountFilter(value: string) {
    setSearchParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        if (value === 'all') {
          next.delete('stressAccount');
        } else {
          next.set('stressAccount', value);
        }
        return next;
      },
      { replace: true }
    );
  }

  function setAssetFilter(value: string) {
    setSearchParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        if (value === 'all') {
          next.delete('stressAsset');
        } else {
          next.set('stressAsset', value);
        }
        return next;
      },
      { replace: true }
    );
  }

  // ─── Filtered portfolio snapshot ───────────────────────────────────────────
  const filteredPortfolio = useMemo(
    () => (portfolio ? applyHoldingFilters(portfolio, accountFilter, assetFilter) : null),
    [portfolio, accountFilter, assetFilter]
  );

  // Detect which FX sliders are relevant to held currencies (scoped to filtered holdings)
  const activeFxSliders = useMemo(() => {
    const currencies = new Set(
      (filteredPortfolio?.holdings ?? []).map((h) => h.currency.toUpperCase())
    );
    return [...currencies]
      .filter((currency) => currency.toUpperCase() !== baseCurrency.toUpperCase())
      .sort()
      .map((currency) => ({
        key: fxShockKey(currency, baseCurrency),
        label: `${currency}/${baseCurrency}`,
      }));
  }, [filteredPortfolio, baseCurrency]);

  useEffect(() => {
    if (presetName === 'Custom') return;
    const preset =
      presetScenarios.find((scenario) => scenario.name === presetName) ?? presetScenarios[0];
    if (preset) setShocks({ ...ZERO_SHOCKS, ...preset.shocks });
  }, [presetName, presetScenarios]);

  // Run stress test whenever shocks or filtered portfolio change (debounced 150ms)
  const scheduleRun = useCallback(
    (nextShocks: ShockMap) => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => {
        const scenario: StressScenario = {
          name: presetName,
          shocks: Object.fromEntries(Object.entries(nextShocks).filter(([, v]) => v !== 0)),
        };
        runTest(scenario, filteredPortfolio);
      }, config.stressTestDebounceMs);
    },
    [filteredPortfolio, presetName, runTest]
  );

  // Initial run on mount / portfolio load / filter change
  useEffect(() => {
    if (filteredPortfolio) scheduleRun(shocks);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filteredPortfolio]);

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

  const isFiltered = accountFilter !== 'all' || assetFilter !== 'all';

  return (
    <div>
      {/* Filter bar */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          marginBottom: 12,
          flexWrap: 'wrap',
        }}
      >
        <span
          style={{
            fontSize: 10,
            color: 'var(--text-muted)',
            fontFamily: 'var(--font-mono)',
            textTransform: 'uppercase',
            letterSpacing: '0.08em',
            marginRight: 4,
          }}
        >
          Filter
        </span>
        <div style={{ width: 160 }}>
          <Select
            value={accountFilter}
            onChange={setAccountFilter}
            options={ACCOUNT_FILTER_OPTIONS}
          />
        </div>
        <div style={{ width: 160 }}>
          <Select value={assetFilter} onChange={setAssetFilter} options={ASSET_FILTER_OPTIONS} />
        </div>
        {isFiltered && (
          <span
            style={{
              fontSize: 10,
              color: 'var(--color-accent)',
              fontFamily: 'var(--font-mono)',
              letterSpacing: '0.06em',
            }}
          >
            {filteredPortfolio?.holdings.length ?? 0} holding
            {(filteredPortfolio?.holdings.length ?? 0) !== 1 ? 's' : ''} selected
          </span>
        )}
        {/* Compare toggle aligned to right */}
        <div style={{ marginLeft: 'auto' }}>
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
      </div>

      {showComparison && filteredPortfolio && (
        <ScenarioComparison portfolio={filteredPortfolio} scenarios={presetScenarioInfo} />
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
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                gap: 12,
                marginBottom: 10,
              }}
            >
              <div style={{ ...SECTION_TITLE, flex: 1, marginBottom: 0 }}>Preset Scenario</div>
              {activePresetInfo && (
                <button
                  onClick={() => setInfoOpen(true)}
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: 6,
                    background: 'transparent',
                    border: '1px solid var(--border-primary)',
                    color: 'var(--text-secondary)',
                    padding: '5px 10px',
                    cursor: 'pointer',
                    fontFamily: 'var(--font-mono)',
                    fontSize: 10,
                    textTransform: 'uppercase',
                    letterSpacing: '0.06em',
                  }}
                >
                  <HelpCircle size={12} />
                  Info
                </button>
              )}
            </div>
            <Select
              value={presetName}
              onChange={handlePresetChange}
              options={presetNames.map((n) => ({ value: n, label: n }))}
            />
            {/* Scenario description */}
            {presetName !== 'Custom' &&
              (() => {
                const preset = presetScenarioInfo.find((s) => s.name === presetName);
                return preset ? (
                  <div
                    style={{
                      marginTop: 8,
                      fontSize: 10,
                      color: 'var(--text-muted)',
                      fontFamily: 'var(--font-sans)',
                      lineHeight: 1.5,
                    }}
                  >
                    {preset.description}
                  </div>
                ) : null;
              })()}
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
                Positive = {baseCurrency} weakens. Foreign holdings convert into more {baseCurrency}
                . Negative = {baseCurrency} strengthens.
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
                  ? 'linear-gradient(135deg, rgba(255,71,87,0.06) 0%, var(--bg-surface) 60%)'
                  : result && result.totalImpact > 1
                    ? 'linear-gradient(135deg, rgba(0,212,170,0.06) 0%, var(--bg-surface) 60%)'
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
                      const d = payload[0]!.payload;
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
            <CollapsiblePanel title="Breakdown" defaultExpanded={true}>
              <div style={{ overflow: 'auto', maxHeight: 360 }}>
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
                        const holding = filteredPortfolio?.holdings.find(
                          (p) => p.id === h.holdingId
                        );
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
            </CollapsiblePanel>
          )}
        </div>
      </div>

      <ResilienceSummary portfolio={filteredPortfolio} />
      <StressTestInfo
        isOpen={infoOpen}
        scenario={activePresetInfo}
        onClose={() => setInfoOpen(false)}
      />

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
