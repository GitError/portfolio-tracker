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
import { usePortfolio } from '../hooks/usePortfolio';
import { useStressTest } from '../hooks/useStressTest';
import {
  createPresetScenarioInfo,
  createPresetScenarios,
  fxShockKey,
  ACCOUNT_OPTIONS,
} from '../lib/constants';
import { formatCurrency, formatPercent, formatCompact } from '../lib/format';
import { pnlColor } from '../lib/colors';
import { EmptyState } from './ui/EmptyState';
import { Select } from './ui/Select';
import { StressTestInfo } from './StressTestInfo';
import { config } from '../lib/config';
import { ShockSliders } from './ShockSliders';
import { PresetScenarioSelector } from './PresetScenarioSelector';
import { StressResultsTable } from './StressResultsTable';
import { ScenarioComparison } from './ScenarioComparison';
import { ResilienceSummary } from './ResilienceSummary';
import type { AccountType, AssetType, PortfolioSnapshot, StressScenario } from '../types/portfolio';

// ─── Shock state keyed as the scenario.shocks keys ───────────────────────────
type ShockMap = Record<string, number>; // values are decimals e.g. -0.20

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

  // Initial run on mount / portfolio load / filter change.
  // shocks and scheduleRun are intentionally omitted from deps: shocks changes are handled
  // synchronously in handleSliderChange/handlePresetChange, and including scheduleRun (which
  // closes over filteredPortfolio) would cause a double invocation on every filter change.
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
          <PresetScenarioSelector
            presetName={presetName}
            presetNames={presetNames}
            scenarioInfo={presetScenarioInfo}
            onSelect={handlePresetChange}
            onInfoOpen={() => setInfoOpen(true)}
          />

          <ShockSliders
            shocks={shocks}
            onChange={handleSliderChange}
            activeFxSliders={activeFxSliders}
            baseCurrency={baseCurrency}
          />

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
            <StressResultsTable
              result={result}
              holdings={filteredPortfolio?.holdings ?? []}
              baseCurrency={baseCurrency}
            />
          )}
        </div>
      </div>

      <ResilienceSummary portfolio={filteredPortfolio} />
      <StressTestInfo
        isOpen={infoOpen}
        scenario={activePresetInfo}
        onClose={() => setInfoOpen(false)}
      />
    </div>
  );
}
