import { useState } from 'react';
import type { ImportResult, PreviewImportResult, PreviewRow } from '../types/portfolio';
import { ASSET_TYPE_CONFIG } from '../lib/constants';
import { Badge } from './ui/Badge';

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onImport: (csvContent: string) => Promise<ImportResult>;
  onPreview: (csvContent: string) => Promise<PreviewImportResult>;
}

const MONO: React.CSSProperties = { fontFamily: 'var(--font-mono)' };

const STATUS_LABEL: Record<string, { label: string; color: string }> = {
  ready: { label: 'Ready', color: 'var(--color-gain)' },
  cash: { label: 'Cash', color: 'var(--color-cash)' },
  duplicate: { label: 'Duplicate', color: 'var(--color-warning)' },
  invalid_symbol: { label: 'Invalid symbol', color: 'var(--color-loss)' },
  validation_failed: { label: 'Check failed', color: 'var(--color-loss)' },
};

function statusCell(status: string) {
  const s = STATUS_LABEL[status] ?? { label: status, color: 'var(--text-muted)' };
  return <span style={{ ...MONO, fontSize: 11, color: s.color, fontWeight: 600 }}>{s.label}</span>;
}

function assetTypeBadge(type: string) {
  const cfg = ASSET_TYPE_CONFIG[type as keyof typeof ASSET_TYPE_CONFIG];
  if (!cfg)
    return <span style={{ ...MONO, fontSize: 10, color: 'var(--text-muted)' }}>{type}</span>;
  return <Badge type={type as 'stock' | 'etf' | 'crypto' | 'cash'} />;
}

function downloadTemplate() {
  const template = [
    'symbol,name,type,account,quantity,cost_basis,currency,exchange',
    'AAPL,Apple Inc.,stock,tfsa,50,142.50,USD,NASDAQ',
    'BMO:CA,Bank of Montreal,stock,rrsp,100,80.00,CAD,TSX',
    'VOO,Vanguard S&P 500 ETF,etf,rrsp,20,380.00,USD,NYSE',
    'BTC-USD,Bitcoin,crypto,taxable,0.5,45000.00,USD,CCC',
    ',US Dollar Cash,cash,taxable,5000,1.00,USD,',
  ].join('\n');

  const blob = new Blob([template], { type: 'text/csv;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = 'portfolio-template.csv';
  link.click();
  URL.revokeObjectURL(url);
}

const TD: React.CSSProperties = {
  padding: '6px 10px',
  borderBottom: '1px solid var(--border-subtle)',
  verticalAlign: 'middle',
};

function PreviewTable({ rows }: { rows: PreviewRow[] }) {
  return (
    <div style={{ border: '1px solid var(--border-primary)', overflowX: 'auto' }}>
      <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 11 }}>
        <thead>
          <tr style={{ background: 'var(--bg-surface-alt)' }}>
            {[
              '#',
              'Symbol (CSV)',
              'Resolved',
              'Name',
              'Type',
              'Exch',
              'CCY',
              'Qty',
              'Cost',
              'Status',
            ].map((h) => (
              <th
                key={h}
                style={{
                  ...TD,
                  ...MONO,
                  textAlign: 'left',
                  color: 'var(--text-muted)',
                  textTransform: 'uppercase',
                  letterSpacing: '0.06em',
                  fontWeight: 400,
                }}
              >
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr
              key={`${r.row}-${r.originalSymbol}`}
              style={{
                background: r.status === 'ready' ? 'transparent' : 'rgba(255,71,87,0.04)',
                opacity:
                  r.status === 'duplicate' ||
                  r.status.startsWith('invalid') ||
                  r.status === 'validation_failed'
                    ? 0.65
                    : 1,
              }}
            >
              <td style={{ ...TD, ...MONO, color: 'var(--text-muted)' }}>{r.row}</td>
              <td style={{ ...TD, ...MONO, color: 'var(--text-secondary)' }}>
                {r.originalSymbol || '—'}
              </td>
              <td style={{ ...TD, ...MONO, color: 'var(--text-primary)', fontWeight: 600 }}>
                {r.resolvedSymbol || '—'}
              </td>
              <td
                style={{
                  ...TD,
                  fontFamily: 'var(--font-sans)',
                  color: 'var(--text-secondary)',
                  maxWidth: 160,
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
              >
                {r.name || '—'}
              </td>
              <td style={TD}>{assetTypeBadge(r.assetType)}</td>
              <td style={{ ...TD, ...MONO, color: 'var(--text-muted)' }}>{r.exchange || '—'}</td>
              <td style={{ ...TD, ...MONO, color: 'var(--text-muted)' }}>{r.currency}</td>
              <td style={{ ...TD, ...MONO, color: 'var(--text-secondary)', textAlign: 'right' }}>
                {r.quantity.toLocaleString()}
              </td>
              <td style={{ ...TD, ...MONO, color: 'var(--text-secondary)', textAlign: 'right' }}>
                {r.costBasis.toFixed(2)}
              </td>
              <td style={TD}>{statusCell(r.status)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function ImportHoldingsModal({ isOpen, onClose, onImport, onPreview }: Props) {
  const [filename, setFilename] = useState('');
  const [csvContent, setCsvContent] = useState('');
  const [previewing, setPreviewing] = useState(false);
  const [preview, setPreview] = useState<PreviewImportResult | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<ImportResult | null>(null);

  async function loadFile(file: File) {
    setError(null);
    setResult(null);
    setPreview(null);
    const text = await file.text();
    setFilename(file.name);
    setCsvContent(text);

    setPreviewing(true);
    try {
      setPreview(await onPreview(text));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setPreviewing(false);
    }
  }

  async function handleImport() {
    if (!csvContent.trim()) {
      setError('Select a CSV file first');
      return;
    }
    setRunning(true);
    setError(null);
    try {
      setResult(await onImport(csvContent));
      setPreview(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setRunning(false);
    }
  }

  if (!isOpen) return null;

  const canImport = !!preview && preview.readyCount > 0 && !running && !previewing;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        background: 'rgba(0,0,0,0.65)',
        backdropFilter: 'blur(4px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1100,
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget && !running && !previewing) onClose();
      }}
    >
      <div
        style={{
          width: '100%',
          maxWidth: 880,
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-primary)',
          padding: 20,
          maxHeight: '88vh',
          overflow: 'auto',
        }}
      >
        {/* Header */}
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'flex-start',
            marginBottom: 16,
          }}
        >
          <div>
            <div
              style={{
                fontFamily: 'var(--font-sans)',
                fontSize: 16,
                fontWeight: 600,
                color: 'var(--text-primary)',
              }}
            >
              Import Holdings
            </div>
            <div style={{ ...MONO, fontSize: 11, color: 'var(--text-muted)', marginTop: 4 }}>
              Supports <code style={{ color: 'var(--text-secondary)' }}>SYMBOL:COUNTRY</code> (e.g.{' '}
              <code style={{ color: 'var(--text-secondary)' }}>BMO:CA</code>), plain symbols, and
              cash rows.
            </div>
          </div>
          <button
            onClick={onClose}
            disabled={running || previewing}
            style={{
              background: 'none',
              border: '1px solid var(--border-primary)',
              color: 'var(--text-secondary)',
              ...MONO,
              fontSize: 11,
              padding: '6px 10px',
              cursor: running || previewing ? 'not-allowed' : 'pointer',
            }}
          >
            Close
          </button>
        </div>

        {/* File picker */}
        <label style={{ display: 'block', marginBottom: 12 }}>
          <div
            style={{
              border: '1px dashed var(--border-primary)',
              background: 'var(--bg-primary)',
              padding: '20px',
              color: 'var(--text-secondary)',
              ...MONO,
              fontSize: 12,
              textAlign: 'center',
              cursor: 'pointer',
            }}
          >
            <div>{filename || 'Choose a .csv file'}</div>
            <div style={{ marginTop: 6, fontSize: 11, color: 'var(--text-muted)' }}>
              Max 500 rows · comma or semicolon delimiters accepted
            </div>
          </div>
          <input
            type="file"
            accept=".csv,text/csv"
            style={{ display: 'none' }}
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) void loadFile(file);
            }}
          />
        </label>

        {/* Action bar */}
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            marginBottom: 14,
          }}
        >
          <button
            onClick={downloadTemplate}
            style={{
              background: 'none',
              border: '1px solid var(--border-primary)',
              color: 'var(--text-secondary)',
              ...MONO,
              fontSize: 11,
              padding: '6px 10px',
              cursor: 'pointer',
            }}
          >
            Download Template
          </button>
          <button
            onClick={() => void handleImport()}
            disabled={!canImport}
            style={{
              background: canImport ? 'var(--color-accent)' : 'var(--border-primary)',
              border: 'none',
              color: canImport ? '#fff' : 'var(--text-muted)',
              fontFamily: 'var(--font-sans)',
              fontSize: 12,
              fontWeight: 600,
              padding: '8px 16px',
              cursor: canImport ? 'pointer' : 'not-allowed',
            }}
          >
            {running
              ? 'Importing…'
              : preview
                ? `Import ${preview.readyCount} row${preview.readyCount !== 1 ? 's' : ''}`
                : 'Import'}
          </button>
        </div>

        {/* Error */}
        {error ? (
          <div
            style={{
              marginBottom: 12,
              border: '1px solid var(--color-loss)',
              color: 'var(--color-loss)',
              background: 'rgba(255,71,87,0.08)',
              padding: '10px 12px',
              fontSize: 12,
              ...MONO,
            }}
          >
            {error}
          </div>
        ) : null}

        {/* Previewing spinner */}
        {previewing ? (
          <div style={{ ...MONO, fontSize: 12, color: 'var(--text-muted)', marginBottom: 12 }}>
            Validating symbols…
          </div>
        ) : null}

        {/* Enriched preview table */}
        {!previewing && preview ? (
          <div style={{ marginBottom: 16 }}>
            <div
              style={{
                ...MONO,
                fontSize: 11,
                color: 'var(--text-muted)',
                textTransform: 'uppercase',
                letterSpacing: '0.08em',
                marginBottom: 8,
                display: 'flex',
                gap: 16,
              }}
            >
              <span>Preview</span>
              <span style={{ color: 'var(--color-gain)' }}>{preview.readyCount} ready</span>
              {preview.skipCount > 0 && (
                <span style={{ color: 'var(--color-loss)' }}>{preview.skipCount} will skip</span>
              )}
            </div>
            <PreviewTable rows={preview.rows} />
          </div>
        ) : null}

        {/* Import result */}
        {result ? (
          <div>
            <div
              style={{
                fontFamily: 'var(--font-sans)',
                fontSize: 14,
                fontWeight: 600,
                color: 'var(--text-primary)',
                marginBottom: 8,
              }}
            >
              Imported {result.imported.length} of {result.totalRows} rows
            </div>
            {result.skipped.length > 0 ? (
              <div style={{ border: '1px solid var(--border-primary)', overflowX: 'auto' }}>
                <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 11 }}>
                  <thead>
                    <tr style={{ background: 'var(--bg-surface-alt)' }}>
                      {['Row', 'Symbol', 'Reason'].map((h) => (
                        <th
                          key={h}
                          style={{
                            ...TD,
                            ...MONO,
                            textAlign: 'left',
                            color: 'var(--text-muted)',
                            textTransform: 'uppercase',
                            letterSpacing: '0.06em',
                            fontWeight: 400,
                          }}
                        >
                          {h}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {result.skipped.map((item) => (
                      <tr key={`${item.row}-${item.symbol}`}>
                        <td style={{ ...TD, ...MONO, color: 'var(--text-muted)' }}>{item.row}</td>
                        <td style={{ ...TD, ...MONO, color: 'var(--text-primary)' }}>
                          {item.symbol || '—'}
                        </td>
                        <td style={{ ...TD, ...MONO, color: 'var(--color-loss)' }}>
                          {STATUS_LABEL[item.reason]?.label ?? item.reason}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div style={{ ...MONO, fontSize: 12, color: 'var(--color-gain)' }}>
                All rows imported successfully.
              </div>
            )}
          </div>
        ) : null}
      </div>
    </div>
  );
}
