import { useState, useMemo } from 'react';
import { Plus, Pencil, Trash2, ChevronUp, ChevronDown, Upload } from 'lucide-react';
import { usePortfolio } from '../hooks/usePortfolio';
import { AddHoldingModal } from './AddHoldingModal';
import { ImportHoldingsModal } from './ImportHoldingsModal';
import { Badge } from './ui/Badge';
import { EmptyState } from './ui/EmptyState';
import { useToast } from './ui/Toast';
import { formatCurrency, formatNumber, formatPercent } from '../lib/format';
import { pnlColor } from '../lib/colors';
import type { Holding, HoldingInput, HoldingWithPrice } from '../types/portfolio';

type SortKey = keyof Pick<
  HoldingWithPrice,
  | 'symbol'
  | 'name'
  | 'assetType'
  | 'quantity'
  | 'costBasis'
  | 'currentPrice'
  | 'marketValueCad'
  | 'gainLoss'
  | 'gainLossPercent'
  | 'weight'
>;

interface SortState {
  key: SortKey;
  dir: 'asc' | 'desc';
}

const COLUMNS: { key: SortKey; label: string; align: 'left' | 'right' }[] = [
  { key: 'symbol', label: 'Symbol', align: 'left' },
  { key: 'name', label: 'Name', align: 'left' },
  { key: 'assetType', label: 'Type', align: 'left' },
  { key: 'quantity', label: 'Qty', align: 'right' },
  { key: 'costBasis', label: 'Cost Basis', align: 'right' },
  { key: 'currentPrice', label: 'Price', align: 'right' },
  { key: 'marketValueCad', label: 'Mkt Value (CAD)', align: 'right' },
  { key: 'gainLoss', label: 'Gain/Loss', align: 'right' },
  { key: 'gainLossPercent', label: 'G/L %', align: 'right' },
  { key: 'weight', label: 'Weight', align: 'right' },
];

const TH: React.CSSProperties = {
  padding: '8px 10px',
  fontSize: 10,
  color: 'var(--text-secondary)',
  fontFamily: 'var(--font-mono)',
  textTransform: 'uppercase',
  letterSpacing: '0.08em',
  fontWeight: 600,
  background: 'var(--bg-surface-alt)',
  borderBottom: '1px solid var(--border-primary)',
  whiteSpace: 'nowrap',
  userSelect: 'none',
  cursor: 'pointer',
};

const TD: React.CSSProperties = {
  padding: '7px 10px',
  fontSize: 12,
  borderBottom: '1px solid var(--border-subtle)',
  borderRight: '1px solid var(--border-subtle)',
  whiteSpace: 'nowrap',
};

export function Holdings() {
  const { portfolio, holdings, addHolding, updateHolding, deleteHolding, importHoldingsCsv } =
    usePortfolio();
  const { showToast } = useToast();
  const [sort, setSort] = useState<SortState>({ key: 'weight', dir: 'desc' });
  const [search, setSearch] = useState('');
  const [modalOpen, setModalOpen] = useState(false);
  const [importOpen, setImportOpen] = useState(false);
  const [editing, setEditing] = useState<Holding | undefined>(undefined);
  const [pendingDelete, setPendingDelete] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const rows: HoldingWithPrice[] = useMemo(() => {
    const source = portfolio?.holdings ?? [];
    const filtered = search
      ? source.filter(
          (h) =>
            h.symbol.toLowerCase().includes(search.toLowerCase()) ||
            h.name.toLowerCase().includes(search.toLowerCase())
        )
      : source;

    return [...filtered].sort((a, b) => {
      const av = a[sort.key];
      const bv = b[sort.key];
      const cmp =
        typeof av === 'string' && typeof bv === 'string'
          ? av.localeCompare(bv)
          : (av as number) - (bv as number);
      return sort.dir === 'asc' ? cmp : -cmp;
    });
  }, [portfolio, sort, search]);

  function toggleSort(key: SortKey) {
    setSort((prev) =>
      prev.key === key ? { key, dir: prev.dir === 'asc' ? 'desc' : 'asc' } : { key, dir: 'desc' }
    );
  }

  async function handleSave(input: HoldingInput) {
    try {
      if (editing) {
        await updateHolding({ ...editing, ...input });
        showToast('Holding updated', 'success');
      } else {
        await addHolding(input);
        showToast('Holding added', 'success');
      }
    } catch (e) {
      showToast(String(e), 'error');
    }
    setEditing(undefined);
  }

  async function handleDelete(id: string) {
    setDeletingId(id);
    try {
      await deleteHolding(id);
      showToast('Holding deleted', 'info');
    } catch (e) {
      showToast(String(e), 'error');
    } finally {
      setDeletingId(null);
      setPendingDelete(null);
    }
  }

  async function handleImport(csvContent: string) {
    const result = await importHoldingsCsv(csvContent);
    if (result.imported.length > 0)
      showToast(`Imported ${result.imported.length} holdings`, 'success');
    if (result.skipped.length > 0) showToast(`Skipped ${result.skipped.length} rows`, 'info');
    return result;
  }

  const totals = useMemo(
    () => ({
      marketValueCad: rows.reduce((s, r) => s + r.marketValueCad, 0),
      gainLoss: rows.reduce((s, r) => s + r.gainLoss, 0),
      gainLossPercent: portfolio
        ? (rows.reduce((s, r) => s + r.gainLoss, 0) /
            Math.max(
              rows.reduce((s, r) => s + r.costValueCad, 0),
              1
            )) *
          100
        : 0,
    }),
    [rows, portfolio]
  );

  const isEmpty = holdings.length === 0;

  return (
    <div>
      {/* Top bar */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          marginBottom: 16,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <span
            style={{
              fontFamily: 'var(--font-sans)',
              fontWeight: 600,
              fontSize: 15,
              color: 'var(--text-primary)',
            }}
          >
            Holdings
          </span>
          <span
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 11,
              color: 'var(--text-muted)',
              background: 'var(--bg-surface)',
              border: '1px solid var(--border-primary)',
              padding: '2px 8px',
              borderRadius: '2px',
            }}
          >
            {holdings.length} positions
          </span>
        </div>
        <div style={{ display: 'flex', gap: 10, alignItems: 'center' }}>
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search symbol..."
            style={{
              background: 'var(--bg-surface)',
              border: '1px solid var(--border-primary)',
              color: 'var(--text-primary)',
              padding: '6px 10px',
              fontSize: 12,
              fontFamily: 'var(--font-mono)',
              borderRadius: '2px',
              outline: 'none',
              width: 160,
            }}
          />
          <button
            onClick={() => setImportOpen(true)}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '6px 12px',
              background: 'var(--bg-surface)',
              border: '1px solid var(--border-primary)',
              color: 'var(--text-primary)',
              borderRadius: '2px',
              fontFamily: 'var(--font-mono)',
              fontSize: 11,
              cursor: 'pointer',
            }}
          >
            <Upload size={12} />
            Import CSV
          </button>
          <button
            onClick={() => {
              setEditing(undefined);
              setModalOpen(true);
            }}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '6px 14px',
              background: 'var(--color-accent)',
              border: 'none',
              color: '#fff',
              borderRadius: '2px',
              fontFamily: 'var(--font-sans)',
              fontSize: 12,
              fontWeight: 600,
              cursor: 'pointer',
            }}
          >
            <Plus size={13} />
            Add Holding
          </button>
        </div>
      </div>

      {isEmpty ? (
        <EmptyState
          message="No positions. Add holdings to begin tracking."
          action={{ label: '+ Add Holding', onClick: () => setModalOpen(true) }}
        />
      ) : (
        <div
          style={{
            border: '1px solid var(--border-primary)',
            overflow: 'auto',
            maxHeight: 'calc(100vh - 200px)',
          }}
        >
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 12 }}>
            <thead style={{ position: 'sticky', top: 0, zIndex: 2 }}>
              <tr>
                {COLUMNS.map(({ key, label, align }) => (
                  <th key={key} onClick={() => toggleSort(key)} style={{ ...TH, textAlign: align }}>
                    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 3 }}>
                      {label}
                      {sort.key === key ? (
                        sort.dir === 'asc' ? (
                          <ChevronUp size={10} />
                        ) : (
                          <ChevronDown size={10} />
                        )
                      ) : null}
                    </span>
                  </th>
                ))}
                <th style={{ ...TH, textAlign: 'center' }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((h, i) => {
                const isDeleting = deletingId === h.id;
                const isPending = pendingDelete === h.id;
                const bg = i % 2 === 0 ? 'var(--bg-surface)' : 'var(--bg-surface-alt)';
                return (
                  <tr
                    key={h.id}
                    style={{
                      background: isPending
                        ? 'rgba(255,71,87,0.08)'
                        : isDeleting
                          ? 'rgba(255,71,87,0.15)'
                          : bg,
                      transition: 'background 200ms',
                    }}
                    onMouseEnter={(e) => {
                      if (!isPending && !isDeleting)
                        (e.currentTarget as HTMLElement).style.background =
                          'var(--bg-surface-hover)';
                    }}
                    onMouseLeave={(e) => {
                      if (!isPending && !isDeleting)
                        (e.currentTarget as HTMLElement).style.background = bg;
                    }}
                  >
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
                    <td
                      style={{
                        ...TD,
                        color: 'var(--text-secondary)',
                        maxWidth: 160,
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                      }}
                    >
                      {h.name}
                    </td>
                    <td style={TD}>
                      <Badge type={h.assetType} />
                    </td>
                    <td
                      style={{
                        ...TD,
                        textAlign: 'right',
                        fontFamily: 'var(--font-mono)',
                        color: 'var(--text-secondary)',
                      }}
                    >
                      {formatNumber(h.quantity, h.assetType === 'crypto' ? 4 : 2)}
                    </td>
                    <td
                      style={{
                        ...TD,
                        textAlign: 'right',
                        fontFamily: 'var(--font-mono)',
                        color: 'var(--text-secondary)',
                      }}
                    >
                      {formatNumber(h.costBasis, 2)} {h.currency}
                    </td>
                    <td
                      style={{
                        ...TD,
                        textAlign: 'right',
                        fontFamily: 'var(--font-mono)',
                        color: 'var(--text-primary)',
                      }}
                    >
                      {h.assetType === 'cash'
                        ? '—'
                        : `${formatNumber(h.currentPrice, 2)} ${h.currency}`}
                    </td>
                    <td
                      style={{
                        ...TD,
                        textAlign: 'right',
                        fontFamily: 'var(--font-mono)',
                        color: 'var(--text-primary)',
                        fontWeight: 600,
                      }}
                    >
                      {formatCurrency(h.marketValueCad)}
                    </td>
                    <td
                      style={{
                        ...TD,
                        textAlign: 'right',
                        fontFamily: 'var(--font-mono)',
                        color: pnlColor(h.gainLoss),
                      }}
                    >
                      {h.gainLoss >= 0 ? '+' : ''}
                      {formatCurrency(h.gainLoss)}
                    </td>
                    <td
                      style={{
                        ...TD,
                        textAlign: 'right',
                        fontFamily: 'var(--font-mono)',
                        color: pnlColor(h.gainLossPercent),
                      }}
                    >
                      {h.assetType === 'cash' ? '—' : formatPercent(h.gainLossPercent)}
                    </td>
                    <td
                      style={{
                        ...TD,
                        textAlign: 'right',
                        fontFamily: 'var(--font-mono)',
                        color: 'var(--text-secondary)',
                      }}
                    >
                      {h.weight.toFixed(1)}%
                    </td>
                    <td style={{ ...TD, textAlign: 'center', borderRight: 'none' }}>
                      {isPending ? (
                        <span style={{ display: 'inline-flex', gap: 6, alignItems: 'center' }}>
                          <span
                            style={{
                              fontSize: 11,
                              color: 'var(--color-loss)',
                              fontFamily: 'var(--font-mono)',
                            }}
                          >
                            Delete?
                          </span>
                          <button
                            onClick={() => handleDelete(h.id)}
                            style={{
                              fontSize: 11,
                              color: 'var(--color-loss)',
                              background: 'none',
                              border: '1px solid var(--color-loss)',
                              padding: '2px 6px',
                              borderRadius: '2px',
                              cursor: 'pointer',
                              fontFamily: 'var(--font-mono)',
                            }}
                          >
                            Confirm
                          </button>
                          <button
                            onClick={() => setPendingDelete(null)}
                            style={{
                              fontSize: 11,
                              color: 'var(--text-secondary)',
                              background: 'none',
                              border: '1px solid var(--border-primary)',
                              padding: '2px 6px',
                              borderRadius: '2px',
                              cursor: 'pointer',
                              fontFamily: 'var(--font-mono)',
                            }}
                          >
                            Cancel
                          </button>
                        </span>
                      ) : (
                        <span style={{ display: 'inline-flex', gap: 8, alignItems: 'center' }}>
                          <button
                            onClick={() => {
                              setEditing(h);
                              setModalOpen(true);
                            }}
                            title="Edit"
                            style={{
                              background: 'none',
                              border: 'none',
                              color: 'var(--text-muted)',
                              cursor: 'pointer',
                              padding: 3,
                              display: 'flex',
                              alignItems: 'center',
                            }}
                          >
                            <Pencil size={13} />
                          </button>
                          <button
                            onClick={() => setPendingDelete(h.id)}
                            title="Delete"
                            style={{
                              background: 'none',
                              border: 'none',
                              color: 'var(--text-muted)',
                              cursor: 'pointer',
                              padding: 3,
                              display: 'flex',
                              alignItems: 'center',
                            }}
                          >
                            <Trash2 size={13} />
                          </button>
                        </span>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
            <tfoot style={{ position: 'sticky', bottom: 0 }}>
              <tr
                style={{
                  background: 'var(--bg-surface-alt)',
                  borderTop: '2px solid var(--border-primary)',
                }}
              >
                <td
                  colSpan={6}
                  style={{
                    ...TD,
                    fontFamily: 'var(--font-mono)',
                    fontWeight: 700,
                    color: 'var(--text-secondary)',
                    fontSize: 10,
                    textTransform: 'uppercase',
                    letterSpacing: '0.06em',
                  }}
                >
                  Total ({rows.length} positions)
                </td>
                <td
                  style={{
                    ...TD,
                    textAlign: 'right',
                    fontFamily: 'var(--font-mono)',
                    fontWeight: 700,
                    color: 'var(--text-primary)',
                    fontSize: 13,
                  }}
                >
                  {formatCurrency(totals.marketValueCad)}
                </td>
                <td
                  style={{
                    ...TD,
                    textAlign: 'right',
                    fontFamily: 'var(--font-mono)',
                    fontWeight: 700,
                    color: pnlColor(totals.gainLoss),
                    fontSize: 13,
                  }}
                >
                  {totals.gainLoss >= 0 ? '+' : ''}
                  {formatCurrency(totals.gainLoss)}
                </td>
                <td
                  style={{
                    ...TD,
                    textAlign: 'right',
                    fontFamily: 'var(--font-mono)',
                    fontWeight: 700,
                    color: pnlColor(totals.gainLossPercent),
                    fontSize: 13,
                  }}
                >
                  {formatPercent(totals.gainLossPercent)}
                </td>
                <td colSpan={2} style={TD} />
              </tr>
            </tfoot>
          </table>
        </div>
      )}

      <AddHoldingModal
        isOpen={modalOpen}
        onClose={() => {
          setModalOpen(false);
          setEditing(undefined);
        }}
        onSave={handleSave}
        editingHolding={editing}
      />
      <ImportHoldingsModal
        isOpen={importOpen}
        onClose={() => setImportOpen(false)}
        onImport={handleImport}
      />
    </div>
  );
}
