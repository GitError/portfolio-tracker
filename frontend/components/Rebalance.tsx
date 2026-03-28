import { useState, useCallback, useEffect } from 'react';
import { Download, RefreshCw, Copy } from 'lucide-react';
import { Spinner } from './ui/Spinner';
import { formatCurrency, formatNumber, formatPercent } from '../lib/format';
import { tauriInvoke } from '../lib/tauri';
import type { RebalanceSuggestion } from '../types/portfolio';

const DEFAULT_DRIFT_THRESHOLD = 5;

function DriftBadge({ drift }: { drift: number }) {
  const isOver = drift > 0;
  const color = isOver ? 'var(--color-loss)' : 'var(--color-accent)';
  const label = isOver ? `+${formatNumber(drift, 2)}pp` : `${formatNumber(drift, 2)}pp`;
  return (
    <span
      style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 13,
        color,
        fontWeight: 600,
      }}
    >
      {label}
    </span>
  );
}

function TradeCell({ suggestion }: { suggestion: RebalanceSuggestion }) {
  const isSell = suggestion.suggestedTradeCad > 0;
  const color = isSell ? 'var(--color-loss)' : 'var(--color-gain)';
  const action = isSell ? 'Sell' : 'Buy';
  const units = Math.abs(suggestion.suggestedUnits);
  return (
    <span
      style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 13,
        color,
        fontWeight: 600,
      }}
    >
      {action} {formatNumber(units, 4)} units
    </span>
  );
}

function buildCsvContent(suggestions: RebalanceSuggestion[]): string {
  const header = [
    'symbol',
    'name',
    'current_weight_%',
    'target_weight_%',
    'drift_pp',
    'action',
    'units',
    'amount_cad',
    'current_price_cad',
  ].join(',');

  const rows = suggestions.map((s) => {
    const action = s.suggestedTradeCad > 0 ? 'sell' : 'buy';
    const units = Math.abs(s.suggestedUnits).toFixed(4);
    const amount = Math.abs(s.suggestedTradeCad).toFixed(2);
    return [
      s.symbol,
      `"${s.name.replace(/"/g, '""')}"`,
      s.currentWeight.toFixed(2),
      s.targetWeight.toFixed(2),
      s.drift.toFixed(2),
      action,
      units,
      amount,
      s.currentPriceCad.toFixed(4),
    ].join(',');
  });

  return [header, ...rows].join('\n');
}

export function Rebalance() {
  const [driftThreshold, setDriftThreshold] = useState(DEFAULT_DRIFT_THRESHOLD);
  const [suggestions, setSuggestions] = useState<RebalanceSuggestion[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const fetchSuggestions = useCallback(async (threshold: number) => {
    setLoading(true);
    setError(null);
    try {
      const result = await tauriInvoke<RebalanceSuggestion[]>('get_rebalance_suggestions', {
        driftThreshold: threshold,
      });
      setSuggestions(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setSuggestions(null);
    } finally {
      setLoading(false);
    }
  }, []);

  const handleThresholdChange = useCallback(
    (value: number) => {
      setDriftThreshold(value);
      void fetchSuggestions(value);
    },
    [fetchSuggestions]
  );

  const handleRefresh = useCallback(() => {
    void fetchSuggestions(driftThreshold);
  }, [fetchSuggestions, driftThreshold]);

  const handleExport = useCallback(() => {
    if (!suggestions || suggestions.length === 0) return;
    const csv = buildCsvContent(suggestions);
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `rebalance-${new Date().toISOString().slice(0, 10)}.csv`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, [suggestions]);

  const handleCopyClipboard = useCallback(async () => {
    if (!suggestions || suggestions.length === 0) return;
    const csv = buildCsvContent(suggestions);
    try {
      await navigator.clipboard.writeText(csv);
    } catch {
      const textarea = document.createElement('textarea');
      textarea.value = csv;
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand('copy');
      document.body.removeChild(textarea);
    }
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [suggestions]);

  // Load on first render
  useEffect(() => {
    void fetchSuggestions(driftThreshold);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // Run once on mount; driftThreshold starts at DEFAULT and handleThresholdChange handles subsequent changes

  return (
    <div
      style={{
        padding: '24px 32px',
        height: '100%',
        overflowY: 'auto',
        boxSizing: 'border-box',
      }}
    >
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          marginBottom: 24,
          flexWrap: 'wrap',
          gap: 12,
        }}
      >
        <div>
          <h1
            style={{
              margin: 0,
              fontSize: 20,
              fontWeight: 600,
              color: 'var(--text-primary)',
              fontFamily: 'var(--font-sans)',
            }}
          >
            Rebalance
          </h1>
          <p
            style={{
              margin: '4px 0 0',
              fontSize: 13,
              color: 'var(--text-secondary)',
              fontFamily: 'var(--font-sans)',
            }}
          >
            Holdings that have drifted from their target allocation
          </p>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          {/* Drift threshold control */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              background: 'var(--bg-surface)',
              border: '1px solid var(--border-primary)',
              borderRadius: 2,
              padding: '6px 12px',
            }}
          >
            <label
              htmlFor="drift-threshold"
              style={{
                fontSize: 12,
                color: 'var(--text-secondary)',
                fontFamily: 'var(--font-sans)',
                whiteSpace: 'nowrap',
              }}
            >
              Show drift &gt;
            </label>
            <input
              id="drift-threshold"
              type="number"
              min={0}
              max={50}
              step={1}
              value={driftThreshold}
              onChange={(e) => {
                const val = Math.max(0, Math.min(50, Number(e.target.value)));
                handleThresholdChange(val);
              }}
              style={{
                width: 52,
                background: 'var(--bg-surface-hover)',
                border: '1px solid var(--border-primary)',
                borderRadius: 2,
                color: 'var(--text-primary)',
                fontFamily: 'var(--font-mono)',
                fontSize: 13,
                padding: '2px 6px',
                outline: 'none',
                textAlign: 'right',
              }}
            />
            <span
              style={{
                fontSize: 12,
                color: 'var(--text-secondary)',
                fontFamily: 'var(--font-sans)',
              }}
            >
              %
            </span>
          </div>

          {/* Refresh */}
          <button
            onClick={handleRefresh}
            disabled={loading}
            title="Refresh rebalance suggestions"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '6px 14px',
              background: 'transparent',
              border: '1px solid var(--border-primary)',
              borderRadius: 2,
              color: 'var(--text-secondary)',
              cursor: loading ? 'not-allowed' : 'pointer',
              fontSize: 13,
              fontFamily: 'var(--font-sans)',
              transition: 'color 150ms, border-color 150ms',
              opacity: loading ? 0.5 : 1,
            }}
          >
            <RefreshCw size={14} />
            Refresh
          </button>

          {/* Copy to clipboard */}
          <button
            onClick={() => void handleCopyClipboard()}
            disabled={!suggestions || suggestions.length === 0}
            title="Copy trade list as CSV to clipboard"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '6px 14px',
              background: 'transparent',
              border: '1px solid var(--border-primary)',
              borderRadius: 2,
              color: 'var(--text-secondary)',
              cursor: !suggestions || suggestions.length === 0 ? 'not-allowed' : 'pointer',
              fontSize: 13,
              fontFamily: 'var(--font-sans)',
              opacity: !suggestions || suggestions.length === 0 ? 0.4 : 1,
              transition: 'opacity 150ms',
            }}
          >
            <Copy size={14} />
            {copied ? 'Copied!' : 'Copy CSV'}
          </button>

          {/* Save CSV file */}
          <button
            onClick={handleExport}
            disabled={!suggestions || suggestions.length === 0}
            title="Save trade list as CSV file"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '6px 14px',
              background: 'var(--color-accent)',
              border: 'none',
              borderRadius: 2,
              color: '#fff',
              cursor: !suggestions || suggestions.length === 0 ? 'not-allowed' : 'pointer',
              fontSize: 13,
              fontFamily: 'var(--font-sans)',
              fontWeight: 500,
              opacity: !suggestions || suggestions.length === 0 ? 0.4 : 1,
              transition: 'opacity 150ms',
            }}
          >
            <Download size={14} />
            Save CSV
          </button>
        </div>
      </div>

      {/* Loading state */}
      {loading && (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            padding: 64,
          }}
        >
          <Spinner size="lg" />
        </div>
      )}

      {/* Error state */}
      {!loading && error && (
        <div
          style={{
            padding: '16px 20px',
            background: 'rgba(255,71,87,0.08)',
            border: '1px solid rgba(255,71,87,0.3)',
            borderRadius: 2,
            color: 'var(--color-loss)',
            fontSize: 13,
            fontFamily: 'var(--font-sans)',
          }}
        >
          {error}
        </div>
      )}

      {/* Empty state — no holdings have target weights */}
      {!loading && !error && suggestions !== null && suggestions.length === 0 && (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            padding: '80px 32px',
            textAlign: 'center',
            color: 'var(--text-secondary)',
          }}
        >
          <div
            style={{
              fontSize: 40,
              marginBottom: 16,
              opacity: 0.4,
            }}
          >
            ⚖
          </div>
          <p
            style={{
              margin: 0,
              fontSize: 15,
              fontWeight: 500,
              color: 'var(--text-primary)',
              fontFamily: 'var(--font-sans)',
            }}
          >
            No rebalancing needed
          </p>
          <p
            style={{
              margin: '8px 0 0',
              fontSize: 13,
              fontFamily: 'var(--font-sans)',
              maxWidth: 400,
            }}
          >
            Set target weights on your holdings to see rebalancing suggestions, or lower the drift
            threshold above.
          </p>
        </div>
      )}

      {/* Results table */}
      {!loading && !error && suggestions !== null && suggestions.length > 0 && (
        <div
          style={{
            border: '1px solid var(--border-primary)',
            overflow: 'hidden',
          }}
        >
          <table
            style={{
              width: '100%',
              borderCollapse: 'collapse',
              tableLayout: 'fixed',
            }}
          >
            <thead>
              <tr
                style={{
                  background: 'var(--bg-surface-alt)',
                  borderBottom: '1px solid var(--border-primary)',
                }}
              >
                {[
                  { label: 'Symbol', width: '14%', align: 'left' as const },
                  { label: 'Name', width: '20%', align: 'left' as const },
                  { label: 'Current %', width: '11%', align: 'right' as const },
                  { label: 'Target %', width: '11%', align: 'right' as const },
                  { label: 'Drift', width: '11%', align: 'right' as const },
                  { label: 'Suggested Trade', width: '20%', align: 'right' as const },
                  { label: 'Amount (CAD)', width: '13%', align: 'right' as const },
                ].map(({ label, width, align }) => (
                  <th
                    key={label}
                    style={{
                      width,
                      padding: '8px 12px',
                      textAlign: align,
                      fontSize: 11,
                      fontWeight: 600,
                      color: 'var(--text-muted)',
                      fontFamily: 'var(--font-sans)',
                      textTransform: 'uppercase',
                      letterSpacing: '0.05em',
                      userSelect: 'none',
                    }}
                  >
                    {label}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {suggestions.map((s, idx) => (
                <tr
                  key={s.holdingId}
                  style={{
                    background: idx % 2 === 0 ? 'var(--bg-surface)' : 'var(--bg-surface-alt)',
                    borderBottom: '1px solid var(--border-subtle)',
                    transition: 'background 100ms',
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLTableRowElement).style.background =
                      'var(--bg-surface-hover)';
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLTableRowElement).style.background =
                      idx % 2 === 0 ? 'var(--bg-surface)' : 'var(--bg-surface-alt)';
                  }}
                >
                  {/* Symbol */}
                  <td
                    style={{
                      padding: '10px 12px',
                      fontFamily: 'var(--font-mono)',
                      fontSize: 13,
                      fontWeight: 600,
                      color: 'var(--text-primary)',
                    }}
                  >
                    {s.symbol}
                  </td>

                  {/* Name */}
                  <td
                    style={{
                      padding: '10px 12px',
                      fontSize: 13,
                      color: 'var(--text-secondary)',
                      fontFamily: 'var(--font-sans)',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                    title={s.name}
                  >
                    {s.name}
                  </td>

                  {/* Current % */}
                  <td
                    style={{
                      padding: '10px 12px',
                      textAlign: 'right',
                      fontFamily: 'var(--font-mono)',
                      fontSize: 13,
                      color: 'var(--text-primary)',
                    }}
                  >
                    {formatPercent(s.currentWeight)}
                  </td>

                  {/* Target % */}
                  <td
                    style={{
                      padding: '10px 12px',
                      textAlign: 'right',
                      fontFamily: 'var(--font-mono)',
                      fontSize: 13,
                      color: 'var(--text-secondary)',
                    }}
                  >
                    {formatPercent(s.targetWeight)}
                  </td>

                  {/* Drift */}
                  <td style={{ padding: '10px 12px', textAlign: 'right' }}>
                    <DriftBadge drift={s.drift} />
                  </td>

                  {/* Suggested Trade */}
                  <td style={{ padding: '10px 12px', textAlign: 'right' }}>
                    <TradeCell suggestion={s} />
                  </td>

                  {/* Amount CAD */}
                  <td
                    style={{
                      padding: '10px 12px',
                      textAlign: 'right',
                      fontFamily: 'var(--font-mono)',
                      fontSize: 13,
                      color: s.suggestedTradeCad > 0 ? 'var(--color-loss)' : 'var(--color-gain)',
                    }}
                  >
                    {formatCurrency(Math.abs(s.suggestedTradeCad))}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Summary row */}
      {!loading && !error && suggestions !== null && suggestions.length > 0 && (
        <div
          style={{
            marginTop: 12,
            fontSize: 12,
            color: 'var(--text-muted)',
            fontFamily: 'var(--font-sans)',
          }}
        >
          {suggestions.length} holding{suggestions.length !== 1 ? 's' : ''} with drift &gt;{' '}
          {driftThreshold}%
        </div>
      )}
    </div>
  );
}
