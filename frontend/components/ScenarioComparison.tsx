// ─── Scenario comparison table for stress test ───────────────────────────────
import { useMemo } from 'react';
import { formatPercent, formatCompact } from '../lib/format';
import { useFormatCurrency } from '../hooks/useFormatters';
import { pnlColor } from '../lib/colors';
import { computeStressImpact } from '../lib/scenarioMath';
import type { PortfolioSnapshot, StressScenarioInfo } from '../types/portfolio';

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

// ─── Props ────────────────────────────────────────────────────────────────────
export interface ScenarioComparisonProps {
  portfolio: PortfolioSnapshot;
  scenarios: StressScenarioInfo[];
}

// ─── ScenarioComparison component ────────────────────────────────────────────
export function ScenarioComparison({ portfolio, scenarios }: ScenarioComparisonProps) {
  const formatCurrency = useFormatCurrency();
  const baseCurrency = portfolio.baseCurrency;

  const rows = useMemo(
    () =>
      scenarios.map((s) => {
        const result = computeStressImpact(portfolio, s);
        const topImpacted = [...result.holdingBreakdown]
          .sort((a, b) => Math.abs(b.impact) - Math.abs(a.impact))
          .slice(0, 2)
          .map((h) => ({ symbol: h.symbol, impact: h.impact }));
        return {
          name: s.name,
          description: s.description,
          stressedValue: result.stressedValue,
          totalImpact: result.totalImpact,
          totalImpactPercent: result.totalImpactPercent,
          topImpacted,
        };
      }),
    [portfolio, scenarios]
  );

  const columns = [
    'Scenario',
    `Stressed Value (${baseCurrency})`,
    `Impact (${baseCurrency})`,
    'Impact (%)',
    'Top Impacted Holdings',
  ];

  return (
    <div style={{ ...PANEL, marginBottom: 1 }}>
      <div style={SECTION_TITLE}>Scenario Comparison</div>
      <div style={{ overflowX: 'auto' }}>
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 11 }}>
          <thead>
            <tr>
              {columns.map((col) => (
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
                            {formatCompact(h.impact, baseCurrency)}
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
