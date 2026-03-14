import { useState, useMemo } from 'react';
import { Plus, Pencil, Trash2, ChevronUp, ChevronDown, Upload, Download } from 'lucide-react';
import { usePortfolio } from '../hooks/usePortfolio';
import { AddHoldingModal } from './AddHoldingModal';
import { ImportHoldingsModal } from './ImportHoldingsModal';
import { Badge } from './ui/Badge';
import { EmptyState } from './ui/EmptyState';
import { useToast } from './ui/Toast';
import { formatCurrency, formatNumber, formatPercent } from '../lib/format';
import { pnlColor } from '../lib/colors';
import { ACCOUNT_OPTIONS } from '../lib/constants';
import type { AccountType, Holding, HoldingInput, HoldingWithPrice } from '../types/portfolio';

type SortKey = keyof Pick<
  HoldingWithPrice,
  | 'symbol'
  | 'name'
  | 'assetType'
  | 'account'
  | 'quantity'
  | 'costBasis'
  | 'currentPrice'
  | 'marketValueCad'
  | 'gainLoss'
  | 'gainLossPercent'
  | 'weight'
  | 'targetWeight'
  | 'targetDeltaPercent'
  | 'targetDeltaValue'
>;

interface SortState {
  key: SortKey;
  dir: 'asc' | 'desc';
}

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
  const {
    portfolio,
    holdings,
    addHolding,
    updateHolding,
    deleteHolding,
    importHoldingsCsv,
    previewImportCsv,
    exportHoldingsCsv,
  } = usePortfolio();
  const { showToast } = useToast();
  const [sort, setSort] = useState<SortState>({ key: 'weight', dir: 'desc' });
  const [search, setSearch] = useState('');
  const [accountFilter, setAccountFilter] = useState<'all' | AccountType>('all');
  const [modalOpen, setModalOpen] = useState(false);
  const [importOpen, setImportOpen] = useState(false);
  const [editing, setEditing] = useState<Holding | undefined>(undefined);
  const [pendingDelete, setPendingDelete] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const baseCurrency = portfolio?.baseCurrency ?? 'CAD';
  const columns: { key: SortKey; label: string; align: 'left' | 'right' }[] = useMemo(
    () => [
      { key: 'symbol', label: 'Symbol', align: 'left' },
      { key: 'name', label: 'Name', align: 'left' },
      { key: 'assetType', label: 'Type', align: 'left' },
      { key: 'account', label: 'Account', align: 'left' },
      { key: 'quantity', label: 'Qty', align: 'right' },
      { key: 'costBasis', label: 'Cost Basis', align: 'right' },
      { key: 'currentPrice', label: 'Price', align: 'right' },
      { key: 'marketValueCad', label: `Mkt Value (${baseCurrency})`, align: 'right' },
      { key: 'weight', label: 'Current %', align: 'right' },
      { key: 'targetWeight', label: 'Target %', align: 'right' },
      { key: 'targetDeltaPercent', label: 'Delta %', align: 'right' },
      { key: 'targetDeltaValue', label: `Rebalance (${baseCurrency})`, align: 'right' },
      { key: 'gainLoss', label: `Gain/Loss (${baseCurrency})`, align: 'right' },
      { key: 'gainLossPercent', label: 'G/L %', align: 'right' },
    ],
    [baseCurrency]
  );

  const rows: HoldingWithPrice[] = useMemo(() => {
    const source = portfolio?.holdings ?? [];
    const filtered = search
      ? source.filter(
          (h) =>
            (accountFilter === 'all' || h.account === accountFilter) &&
            (h.symbol.toLowerCase().includes(search.toLowerCase()) ||
              h.name.toLowerCase().includes(search.toLowerCase()))
        )
      : accountFilter === 'all'
        ? source
        : source.filter((h) => h.account === accountFilter);

    return [...filtered].sort((a, b) => {
      const av = a[sort.key];
      const bv = b[sort.key];
      const cmp =
        typeof av === 'string' && typeof bv === 'string'
          ? av.localeCompare(bv)
          : (av as number) - (bv as number);
      return sort.dir === 'asc' ? cmp : -cmp;
    });
  }, [portfolio, sort, search, accountFilter]);

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
    if (result.imported.length > 0) {
      showToast(`Imported ${result.imported.length} holdings`, 'success');
    }
    if (result.skipped.length > 0) {
      showToast(`Skipped ${result.skipped.length} rows`, 'info');
    }
    return result;
  }

  async function handleExport() {
    try {
      const csv = await exportHoldingsCsv();
      const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = 'holdings-export.csv';
      link.click();
      URL.revokeObjectURL(url);
      showToast('Holdings exported', 'success');
    } catch (e) {
      showToast(String(e), 'error');
    }
  }

  const totals = useMemo(
    () => ({
      marketValueCad: rows.reduce((s, r) => s + r.marketValueCad, 0),
      gainLoss: rows.reduce((s, r) => s + r.gainLoss, 0),
      targetWeight: rows.reduce((s, r) => s + r.targetWeight, 0),
      targetDeltaValue: rows.reduce((s, r) => s + r.targetDeltaValue, 0),
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

  const rebalanceSummary = useMemo(() => {
    if (!portfolio) return null;
    const buys = rows
      .filter((row) => row.targetDeltaValue > 0.01)
      .reduce((sum, row) => sum + row.targetDeltaValue, 0);
    const sells = rows
      .filter((row) => row.targetDeltaValue < -0.01)
      .reduce((sum, row) => sum + Math.abs(row.targetDeltaValue), 0);
    return {
      totalTargetWeight: rows.reduce((sum, row) => sum + row.targetWeight, 0),
      unassignedTargetWeight: Math.max(
        0,
        100 - rows.reduce((sum, row) => sum + row.targetWeight, 0)
      ),
      deployableCash: rows
        .filter((row) => row.assetType === 'cash')
        .reduce((sum, row) => sum + Math.max(0, -row.targetDeltaValue), 0),
      suggestedBuys: buys,
      suggestedSells: sells,
    };
  }, [portfolio, rows]);

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
          <select
            value={accountFilter}
            onChange={(e) => setAccountFilter(e.target.value as 'all' | AccountType)}
            style={{
              background: 'var(--bg-surface)',
              border: '1px solid var(--border-primary)',
              color: 'var(--text-primary)',
              padding: '6px 10px',
              fontSize: 11,
              fontFamily: 'var(--font-mono)',
              borderRadius: '2px',
            }}
          >
            <option value="all">All Accounts</option>
            {ACCOUNT_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          <button
            onClick={() => void handleExport()}
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
            <Download size={12} />
            Export CSV
          </button>
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
        <>
          {rebalanceSummary && (
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(4, minmax(0, 1fr))',
                gap: 1,
                background: 'var(--border-primary)',
                border: '1px solid var(--border-primary)',
                marginBottom: 12,
              }}
            >
              {[
                {
                  label: 'Assigned Target',
                  value: `${rebalanceSummary.totalTargetWeight.toFixed(1)}%`,
                  tone: 'var(--text-primary)',
                },
                {
                  label: 'Unassigned Target',
                  value: `${rebalanceSummary.unassignedTargetWeight.toFixed(1)}%`,
                  tone: 'var(--text-secondary)',
                },
                {
                  label: 'Deployable Cash',
                  value: formatCurrency(rebalanceSummary.deployableCash, baseCurrency),
                  tone: 'var(--color-accent)',
                },
                {
                  label: 'Suggested Net Buys',
                  value: formatCurrency(rebalanceSummary.suggestedBuys, baseCurrency),
                  tone: 'var(--color-gain)',
                },
              ].map((item) => (
                <div
                  key={item.label}
                  style={{
                    background: 'var(--bg-surface)',
                    padding: '12px 14px',
                  }}
                >
                  <div
                    style={{
                      fontFamily: 'var(--font-mono)',
                      fontSize: 10,
                      color: 'var(--text-muted)',
                      textTransform: 'uppercase',
                      letterSpacing: '0.08em',
                      marginBottom: 6,
                    }}
                  >
                    {item.label}
                  </div>
                  <div
                    style={{
                      fontFamily: 'var(--font-mono)',
                      fontSize: 15,
                      fontWeight: 700,
                      color: item.tone,
                    }}
                  >
                    {item.value}
                  </div>
                </div>
              ))}
            </div>
          )}
          <div
            style={{
              border: '1px solid var(--border-primary)',
              overflow: 'auto',
              maxHeight: 'calc(100vh - 260px)',
            }}
          >
            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 12 }}>
              <thead style={{ position: 'sticky', top: 0, zIndex: 2 }}>
                <tr>
                  {columns.map(({ key, label, align }) => (
                    <th
                      key={key}
                      onClick={() => toggleSort(key)}
                      style={{ ...TH, textAlign: align }}
                    >
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
                          color: 'var(--text-secondary)',
                          fontFamily: 'var(--font-mono)',
                          textTransform: 'uppercase',
                        }}
                      >
                        {ACCOUNT_OPTIONS.find((option) => option.value === h.account)?.label ??
                          h.account}
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
                        {formatCurrency(h.marketValueCad, baseCurrency)}
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
                      <td
                        style={{
                          ...TD,
                          textAlign: 'right',
                          fontFamily: 'var(--font-mono)',
                          color: h.targetWeight > 0 ? 'var(--text-primary)' : 'var(--text-muted)',
                        }}
                      >
                        {h.targetWeight > 0 ? `${h.targetWeight.toFixed(1)}%` : '—'}
                      </td>
                      <td
                        style={{
                          ...TD,
                          textAlign: 'right',
                          fontFamily: 'var(--font-mono)',
                          color: pnlColor(h.targetDeltaPercent),
                        }}
                      >
                        {h.targetWeight > 0 || h.assetType === 'cash'
                          ? formatPercent(h.targetDeltaPercent)
                          : '—'}
                      </td>
                      <td
                        style={{
                          ...TD,
                          textAlign: 'right',
                          fontFamily: 'var(--font-mono)',
                          color: pnlColor(h.targetDeltaValue),
                          fontWeight: 600,
                        }}
                      >
                        {h.targetWeight > 0 || h.assetType === 'cash'
                          ? `${h.targetDeltaValue >= 0 ? '+' : ''}${formatCurrency(h.targetDeltaValue, baseCurrency)}`
                          : '—'}
                      </td>
                      <td
                        style={{
                          ...TD,
                          textAlign: 'right',
                          fontFamily: 'var(--font-mono)',
                          color:
                            h.assetType === 'cash' ? 'var(--text-muted)' : pnlColor(h.gainLoss),
                        }}
                      >
                        {h.assetType === 'cash'
                          ? '—'
                          : `${h.gainLoss >= 0 ? '+' : ''}${formatCurrency(h.gainLoss, baseCurrency)}`}
                      </td>
                      <td
                        style={{
                          ...TD,
                          textAlign: 'right',
                          fontFamily: 'var(--font-mono)',
                          color:
                            h.assetType === 'cash'
                              ? 'var(--text-muted)'
                              : pnlColor(h.gainLossPercent),
                        }}
                      >
                        {h.assetType === 'cash' ? '—' : formatPercent(h.gainLossPercent)}
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
                    colSpan={7}
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
                    {formatCurrency(totals.marketValueCad, baseCurrency)}
                  </td>
                  <td
                    style={{
                      ...TD,
                      textAlign: 'right',
                      fontFamily: 'var(--font-mono)',
                      fontWeight: 700,
                      color: 'var(--text-secondary)',
                      fontSize: 13,
                    }}
                  >
                    100.0%
                  </td>
                  <td
                    style={{
                      ...TD,
                      textAlign: 'right',
                      fontFamily: 'var(--font-mono)',
                      fontWeight: 700,
                      color: totals.targetWeight > 0 ? 'var(--text-primary)' : 'var(--text-muted)',
                      fontSize: 13,
                    }}
                  >
                    {totals.targetWeight > 0 ? `${totals.targetWeight.toFixed(1)}%` : '—'}
                  </td>
                  <td
                    style={{
                      ...TD,
                      textAlign: 'right',
                      fontFamily: 'var(--font-mono)',
                      fontWeight: 700,
                      color: pnlColor(100 - totals.targetWeight),
                      fontSize: 13,
                    }}
                  >
                    {formatPercent(totals.targetWeight - 100)}
                  </td>
                  <td
                    style={{
                      ...TD,
                      textAlign: 'right',
                      fontFamily: 'var(--font-mono)',
                      fontWeight: 700,
                      color: pnlColor(totals.targetDeltaValue),
                      fontSize: 13,
                    }}
                  >
                    {totals.targetDeltaValue >= 0 ? '+' : ''}
                    {formatCurrency(totals.targetDeltaValue, baseCurrency)}
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
                    {formatCurrency(totals.gainLoss, baseCurrency)}
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
        </>
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
        onPreview={previewImportCsv}
      />
    </div>
  );
}
