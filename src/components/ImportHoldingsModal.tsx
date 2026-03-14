import { useMemo, useState } from 'react';
import type { ImportResult } from '../types/portfolio';

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onImport: (csvContent: string) => Promise<ImportResult>;
}

const DROP_STYLE: React.CSSProperties = {
  border: '1px dashed var(--border-primary)',
  background: 'var(--bg-primary)',
  padding: '24px 20px',
  color: 'var(--text-secondary)',
  fontFamily: 'var(--font-mono)',
  fontSize: 12,
  textAlign: 'center',
  cursor: 'pointer',
};

function parsePreview(csvContent: string): string[][] {
  return csvContent
    .replace(/^\uFEFF/, '')
    .split(/\r?\n/)
    .filter((line) => line.trim().length > 0)
    .slice(0, 6)
    .map((line) => line.split(/[;,]/).map((cell) => cell.trim()));
}

function downloadTemplate() {
  const template = [
    'symbol,name,type,account,quantity,cost_basis,currency',
    'AAPL,Apple Inc.,stock,tfsa,50,142.50,USD',
    'VOO,Vanguard S&P 500 ETF,etf,rrsp,100,380.00,USD',
    'BTC-CAD,Bitcoin,crypto,taxable,0.5,45000.00,CAD',
  ].join('\n');

  const blob = new Blob([template], { type: 'text/csv;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = 'portfolio-template.csv';
  link.click();
  URL.revokeObjectURL(url);
}

export function ImportHoldingsModal({ isOpen, onClose, onImport }: Props) {
  const [filename, setFilename] = useState('');
  const [csvContent, setCsvContent] = useState('');
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<ImportResult | null>(null);

  const previewRows = useMemo(() => parsePreview(csvContent), [csvContent]);

  async function loadFile(file: File) {
    setError(null);
    setResult(null);
    setFilename(file.name);
    const text = await file.text();
    setCsvContent(text);
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
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setRunning(false);
    }
  }

  if (!isOpen) return null;

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
      onClick={(event) => {
        if (event.target === event.currentTarget && !running) onClose();
      }}
    >
      <div
        style={{
          width: '100%',
          maxWidth: 760,
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-primary)',
          padding: 20,
          maxHeight: '85vh',
          overflow: 'auto',
        }}
      >
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
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
            <div
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 11,
                color: 'var(--text-muted)',
                marginTop: 4,
              }}
            >
              Upload a CSV with `symbol,name,type,account,quantity,cost_basis,currency`
            </div>
          </div>
          <button
            onClick={onClose}
            disabled={running}
            style={{
              background: 'none',
              border: '1px solid var(--border-primary)',
              color: 'var(--text-secondary)',
              fontFamily: 'var(--font-mono)',
              fontSize: 11,
              padding: '6px 10px',
              cursor: running ? 'not-allowed' : 'pointer',
            }}
          >
            Close
          </button>
        </div>

        <label style={{ display: 'block' }}>
          <div style={DROP_STYLE}>
            <div>{filename || 'Choose a .csv file or drop one here'}</div>
            <div style={{ marginTop: 8, fontSize: 11, color: 'var(--text-muted)' }}>
              Max 500 rows. Comma or semicolon delimiters accepted.
            </div>
          </div>
          <input
            type="file"
            accept=".csv,text/csv"
            style={{ display: 'none' }}
            onChange={(event) => {
              const file = event.target.files?.[0];
              if (file) void loadFile(file);
            }}
          />
        </label>

        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            marginTop: 12,
            marginBottom: 12,
          }}
        >
          <button
            onClick={downloadTemplate}
            style={{
              background: 'none',
              border: '1px solid var(--border-primary)',
              color: 'var(--text-secondary)',
              fontFamily: 'var(--font-mono)',
              fontSize: 11,
              padding: '6px 10px',
              cursor: 'pointer',
            }}
          >
            Download Template
          </button>
          <button
            onClick={() => void handleImport()}
            disabled={running || !csvContent.trim()}
            style={{
              background: 'var(--color-accent)',
              border: 'none',
              color: '#fff',
              fontFamily: 'var(--font-sans)',
              fontSize: 12,
              fontWeight: 600,
              padding: '8px 14px',
              cursor: running || !csvContent.trim() ? 'not-allowed' : 'pointer',
              opacity: running || !csvContent.trim() ? 0.6 : 1,
            }}
          >
            {running ? 'Importing...' : 'Import'}
          </button>
        </div>

        {error ? (
          <div
            style={{
              marginBottom: 12,
              border: '1px solid var(--color-loss)',
              color: 'var(--color-loss)',
              background: 'rgba(255,71,87,0.08)',
              padding: '10px 12px',
              fontSize: 12,
              fontFamily: 'var(--font-mono)',
            }}
          >
            {error}
          </div>
        ) : null}

        {previewRows.length > 0 ? (
          <div style={{ marginBottom: 16 }}>
            <div
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 11,
                color: 'var(--text-muted)',
                marginBottom: 8,
                textTransform: 'uppercase',
                letterSpacing: '0.08em',
              }}
            >
              Preview
            </div>
            <div style={{ border: '1px solid var(--border-primary)', overflowX: 'auto' }}>
              <table style={{ width: '100%', borderCollapse: 'collapse' }}>
                <tbody>
                  {previewRows.map((cells, rowIndex) => (
                    <tr
                      key={`${rowIndex}-${cells.join('|')}`}
                      style={{
                        background: rowIndex === 0 ? 'var(--bg-surface-alt)' : 'var(--bg-surface)',
                      }}
                    >
                      {cells.map((cell, cellIndex) => (
                        <td
                          key={`${rowIndex}-${cellIndex}`}
                          style={{
                            padding: '7px 9px',
                            borderBottom: '1px solid var(--border-subtle)',
                            borderRight: '1px solid var(--border-subtle)',
                            fontFamily: 'var(--font-mono)',
                            fontSize: 11,
                            color: rowIndex === 0 ? 'var(--text-primary)' : 'var(--text-secondary)',
                          }}
                        >
                          {cell}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        ) : null}

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
                <table style={{ width: '100%', borderCollapse: 'collapse' }}>
                  <thead>
                    <tr style={{ background: 'var(--bg-surface-alt)' }}>
                      <th style={{ padding: '8px 10px', textAlign: 'left', fontSize: 11 }}>Row</th>
                      <th style={{ padding: '8px 10px', textAlign: 'left', fontSize: 11 }}>
                        Symbol
                      </th>
                      <th style={{ padding: '8px 10px', textAlign: 'left', fontSize: 11 }}>
                        Reason
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {result.skipped.map((item) => (
                      <tr key={`${item.row}-${item.symbol}-${item.reason}`}>
                        <td
                          style={{
                            padding: '7px 10px',
                            borderTop: '1px solid var(--border-subtle)',
                            fontFamily: 'var(--font-mono)',
                            fontSize: 11,
                            color: 'var(--text-secondary)',
                          }}
                        >
                          {item.row}
                        </td>
                        <td
                          style={{
                            padding: '7px 10px',
                            borderTop: '1px solid var(--border-subtle)',
                            fontFamily: 'var(--font-mono)',
                            fontSize: 11,
                            color: 'var(--text-primary)',
                          }}
                        >
                          {item.symbol || '—'}
                        </td>
                        <td
                          style={{
                            padding: '7px 10px',
                            borderTop: '1px solid var(--border-subtle)',
                            fontFamily: 'var(--font-mono)',
                            fontSize: 11,
                            color: 'var(--color-loss)',
                          }}
                        >
                          {item.reason}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 12,
                  color: 'var(--color-gain)',
                }}
              >
                No rows were skipped.
              </div>
            )}
          </div>
        ) : null}
      </div>
    </div>
  );
}
