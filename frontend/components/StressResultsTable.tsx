// ─── Breakdown table for stress test results ─────────────────────────────────
import { ASSET_TYPE_CONFIG } from '../lib/constants';
import { formatCurrency, formatPercent } from '../lib/format';
import { pnlColor } from '../lib/colors';
import { CollapsiblePanel } from './ui/CollapsiblePanel';
import type { HoldingWithPrice, StressResult } from '../types/portfolio';

const TD: React.CSSProperties = {
  padding: '6px 10px',
  fontSize: 11,
  borderBottom: '1px solid var(--border-subtle)',
  borderRight: '1px solid var(--border-subtle)',
  whiteSpace: 'nowrap',
};

// ─── Props ────────────────────────────────────────────────────────────────────
export interface StressResultsTableProps {
  result: StressResult;
  holdings: HoldingWithPrice[];
  baseCurrency: string;
}

// ─── StressResultsTable component ────────────────────────────────────────────
export function StressResultsTable({ result, holdings, baseCurrency }: StressResultsTableProps) {
  const columns = [
    'Symbol',
    'Type',
    `Current Value (${baseCurrency})`,
    'Shock',
    `Stressed Value (${baseCurrency})`,
    `Impact (${baseCurrency})`,
    'Impact (%)',
  ];

  return (
    <CollapsiblePanel title="Breakdown" defaultExpanded={true}>
      <div style={{ overflow: 'auto', maxHeight: 360 }}>
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 11 }}>
          <thead style={{ position: 'sticky', top: 0 }}>
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
                const holding = holdings.find((p) => p.id === h.holdingId);
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
                      {h.impact !== 0 ? formatPercent((h.impact / h.currentValue) * 100) : '—'}
                    </td>
                  </tr>
                );
              })}
          </tbody>
        </table>
      </div>
    </CollapsiblePanel>
  );
}
