import { useState, useMemo, useEffect, useCallback, useRef } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  Plus,
  Pencil,
  Trash2,
  ChevronUp,
  ChevronDown,
  Upload,
  Download,
  Clock,
  Layers,
} from 'lucide-react';
import { usePortfolio } from '../hooks/usePortfolio';
import { useConfig } from '../hooks/useConfig';
import { AddHoldingModal } from './AddHoldingModal';
import { AddTransactionModal } from './AddTransactionModal';
import { ImportHoldingsModal } from './ImportHoldingsModal';
import { AccountBadge, Badge, ExchangeBadge } from './ui/Badge';
import { EmptyState } from './ui/EmptyState';
import { Select } from './ui/Select';
import { useToast } from './ui/Toast';
import {
  formatCurrency,
  formatMonthYear,
  formatNumber,
  formatPercent,
  formatShortDate,
  isPriceStale,
} from '../lib/format';
import { pnlColor } from '../lib/colors';
import { ACCOUNT_OPTIONS, ACCOUNT_TYPE_CONFIG } from '../lib/constants';
import type {
  AccountType,
  Holding,
  HoldingInput,
  HoldingWithPrice,
  PriceData,
} from '../types/portfolio';
import { isTauri, tauriInvoke } from '../lib/tauri';

type SortKey = keyof Pick<
  HoldingWithPrice,
  | 'symbol'
  | 'name'
  | 'assetType'
  | 'account'
  | 'exchange'
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

interface HoldingsProps {
  /** Called by parent (e.g. via keyboard shortcut) to open the Add Holding modal */
  onOpenAddModal?: (handler: () => void) => void;
  /** Called by parent (e.g. via keyboard shortcut) to trigger CSV export */
  onExportRef?: (handler: () => void) => void;
}

export function Holdings({ onOpenAddModal, onExportRef }: HoldingsProps) {
  const { t } = useTranslation();
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
  const [searchParams, setSearchParams] = useSearchParams();

  const search = searchParams.get('search') ?? '';
  const accountFilter = (searchParams.get('account') ?? 'all') as 'all' | AccountType;
  const sortKey = (searchParams.get('sort') ?? 'weight') as SortKey;
  const sortDir = (searchParams.get('dir') ?? 'desc') as 'asc' | 'desc';
  const sort: SortState = useMemo(() => ({ key: sortKey, dir: sortDir }), [sortKey, sortDir]);

  function setSearch(value: string) {
    setSearchParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        if (value === '') {
          next.delete('search');
        } else {
          next.set('search', value);
        }
        return next;
      },
      { replace: true }
    );
  }

  function setAccountFilter(value: 'all' | AccountType) {
    setSearchParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        if (value === 'all') {
          next.delete('account');
        } else {
          next.set('account', value);
        }
        return next;
      },
      { replace: true }
    );
  }

  function setSort(updater: SortState | ((prev: SortState) => SortState)) {
    const next = typeof updater === 'function' ? updater(sort) : updater;
    setSearchParams(
      (prev) => {
        const params = new URLSearchParams(prev);
        if (next.key === 'weight') {
          params.delete('sort');
        } else {
          params.set('sort', next.key);
        }
        if (next.dir === 'desc') {
          params.delete('dir');
        } else {
          params.set('dir', next.dir);
        }
        return params;
      },
      { replace: true }
    );
  }

  const [modalOpen, setModalOpen] = useState(false);
  const [importOpen, setImportOpen] = useState(false);
  const [editing, setEditing] = useState<Holding | undefined>(undefined);
  const [pendingDelete, setPendingDelete] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [bulkDeletePending, setBulkDeletePending] = useState(false);
  const [bulkDeleting, setBulkDeleting] = useState(false);
  const [txModalHolding, setTxModalHolding] = useState<Holding | undefined>(undefined);
  const [groupByAccount, setGroupByAccount] = useState(false);
  const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(new Set());
  const [colPickerOpen, setColPickerOpen] = useState(false);
  const colPickerRef = useRef<HTMLDivElement>(null);
  const { value: hiddenColumnsRaw, setValue: setHiddenColumnsRaw } = useConfig(
    'holdings_hidden_columns',
    JSON.stringify([
      'prevClose',
      'dayOpen',
      'openDate',
      'maturityDate',
      'weight',
      'targetWeight',
      'targetDeltaPercent',
      'targetDeltaValue',
      'exchange',
      'gainLossPercent',
    ])
  );
  const hiddenColumns = useMemo<Set<string>>(() => {
    try {
      return new Set(JSON.parse(hiddenColumnsRaw) as string[]);
    } catch {
      return new Set();
    }
  }, [hiddenColumnsRaw]);
  const [priceMap, setPriceMap] = useState<Record<string, PriceData>>({});

  // Auto-open the add-holding modal when navigated here via keyboard shortcut (?add=1)
  useEffect(() => {
    if (searchParams.get('add') === '1') {
      setModalOpen(true);
      setSearchParams(
        (prev) => {
          const next = new URLSearchParams(prev);
          next.delete('add');
          return next;
        },
        { replace: true }
      );
    }
  }, [searchParams, setSearchParams]);

  const baseCurrency = portfolio?.baseCurrency ?? 'CAD';
  const columns: { key: SortKey; label: string; align: 'left' | 'right' }[] = useMemo(
    () => [
      { key: 'symbol', label: t('holdings.columns.symbol'), align: 'left' },
      { key: 'name', label: t('holdings.columns.name'), align: 'left' },
      { key: 'assetType', label: t('holdings.columns.type'), align: 'left' },
      { key: 'account', label: t('holdings.columns.account'), align: 'left' },
      { key: 'exchange', label: t('holdings.columns.exchange'), align: 'left' },
      { key: 'quantity', label: t('holdings.columns.quantity'), align: 'right' },
      { key: 'costBasis', label: t('holdings.columns.costBasis'), align: 'right' },
      { key: 'currentPrice', label: t('holdings.columns.currentPrice'), align: 'right' },
      {
        key: 'marketValueCad',
        label: `${t('holdings.columns.marketValue')} (${baseCurrency})`,
        align: 'right',
      },
      { key: 'weight', label: 'Current %', align: 'right' },
      { key: 'targetWeight', label: t('holdings.columns.targetWeight'), align: 'right' },
      { key: 'targetDeltaPercent', label: 'Delta %', align: 'right' },
      { key: 'targetDeltaValue', label: `Rebalance (${baseCurrency})`, align: 'right' },
      {
        key: 'gainLoss',
        label: `${t('holdings.columns.gainLoss')} (${baseCurrency})`,
        align: 'right',
      },
      { key: 'gainLossPercent', label: 'G/L %', align: 'right' },
    ],
    [baseCurrency, t]
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
    setSelected(new Set());
  }

  function handleAccountFilterChange(value: 'all' | AccountType) {
    setAccountFilter(value);
    setSelected(new Set());
  }

  function toggleRow(id: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }

  function toggleSelectAll() {
    if (selected.size === rows.length && rows.length > 0) {
      setSelected(new Set());
    } else {
      setSelected(new Set(rows.map((r) => r.id)));
    }
  }

  async function handleBulkDelete() {
    setBulkDeleting(true);
    setIsDeleting(true);
    const ids = Array.from(selected);
    try {
      for (const id of ids) {
        await deleteHolding(id);
      }
      showToast(`Deleted ${ids.length} holdings`, 'info');
      setSelected(new Set());
    } catch (e) {
      showToast(String(e), 'error');
    } finally {
      setBulkDeleting(false);
      setIsDeleting(false);
      setBulkDeletePending(false);
    }
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
    // Guard: only delete holdings that are currently visible in the filtered view.
    const isVisible = rows.some((h) => h.id === id);
    if (!isVisible) {
      setPendingDelete(null);
      showToast('Holding is no longer visible — clear filters and try again', 'error');
      return;
    }
    setDeletingId(id);
    setIsDeleting(true);
    try {
      await deleteHolding(id);
      showToast('Holding deleted', 'info');
    } catch (e) {
      showToast(String(e), 'error');
    } finally {
      setDeletingId(null);
      setIsDeleting(false);
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

  const handleExport = useCallback(async () => {
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
  }, [exportHoldingsCsv, showToast]);

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

  // Grouped view: group rows by account name
  const groupedRows = useMemo(() => {
    if (!groupByAccount) return null;
    const groups: Map<string, HoldingWithPrice[]> = new Map();
    for (const row of rows) {
      const key = row.account;
      const existing = groups.get(key) ?? [];
      groups.set(key, [...existing, row]);
    }
    return Array.from(groups.entries()).map(([account, accountHoldings]) => {
      const totalValue = accountHoldings.reduce((s, h) => s + h.marketValueCad, 0);
      const totalGainLoss = accountHoldings.reduce((s, h) => s + h.gainLoss, 0);
      const totalWeight = accountHoldings.reduce((s, h) => s + h.weight, 0);
      return { account, holdings: accountHoldings, totalValue, totalGainLoss, totalWeight };
    });
  }, [groupByAccount, rows]);

  function toggleGroup(account: string) {
    setCollapsedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(account)) {
        next.delete(account);
      } else {
        next.add(account);
      }
      return next;
    });
  }

  // Load cached price data for open/prevClose columns
  useEffect(() => {
    if (!isTauri()) return;
    tauriInvoke<PriceData[]>('get_cached_prices')
      .then((prices) => {
        const map: Record<string, PriceData> = {};
        for (const p of prices) {
          map[p.symbol] = p;
        }
        setPriceMap(map);
      })
      .catch(() => {
        /* best-effort */
      });
  }, [portfolio]);

  const isEmpty = holdings.length === 0;

  // Register imperative handles so parent can trigger open/export via keyboard shortcuts
  useEffect(() => {
    if (onOpenAddModal) {
      onOpenAddModal(() => {
        setEditing(undefined);
        setModalOpen(true);
      });
    }
  }, [onOpenAddModal]);

  useEffect(() => {
    if (onExportRef) {
      onExportRef(() => {
        void handleExport();
      });
    }
  }, [onExportRef, handleExport]);

  useEffect(() => {
    if (!colPickerOpen) return;
    function handleOutside(e: MouseEvent) {
      if (colPickerRef.current && !colPickerRef.current.contains(e.target as Node)) {
        setColPickerOpen(false);
      }
    }
    document.addEventListener('mousedown', handleOutside);
    return () => document.removeEventListener('mousedown', handleOutside);
  }, [colPickerOpen]);

  const ALWAYS_VISIBLE_COLS = new Set(['symbol', 'marketValueCad']);

  const ALL_COLUMNS: { key: string; label: string; align: 'left' | 'right' }[] = [
    { key: 'symbol', label: t('holdings.columns.symbol'), align: 'left' },
    { key: 'name', label: t('holdings.columns.name'), align: 'left' },
    { key: 'assetType', label: t('holdings.columns.type'), align: 'left' },
    { key: 'account', label: t('holdings.columns.account'), align: 'left' },
    { key: 'exchange', label: t('holdings.columns.exchange'), align: 'left' },
    { key: 'quantity', label: t('holdings.columns.quantity'), align: 'right' },
    { key: 'costBasis', label: t('holdings.columns.costBasis'), align: 'right' },
    { key: 'currentPrice', label: t('holdings.columns.currentPrice'), align: 'right' },
    {
      key: 'marketValueCad',
      label: `${t('holdings.columns.marketValue')} (${baseCurrency})`,
      align: 'right',
    },
    { key: 'weight', label: 'Current %', align: 'right' },
    { key: 'targetWeight', label: t('holdings.columns.targetWeight'), align: 'right' },
    { key: 'targetDeltaPercent', label: 'Delta %', align: 'right' },
    { key: 'targetDeltaValue', label: `Rebalance (${baseCurrency})`, align: 'right' },
    {
      key: 'gainLoss',
      label: `${t('holdings.columns.gainLoss')} (${baseCurrency})`,
      align: 'right',
    },
    { key: 'gainLossPercent', label: 'G/L %', align: 'right' },
    { key: 'prevClose', label: 'Prev Close', align: 'right' },
    { key: 'dayOpen', label: 'Day Open', align: 'right' },
    { key: 'openDate', label: 'Open Date', align: 'right' },
    { key: 'maturityDate', label: 'Maturity', align: 'right' },
  ];

  function toggleHiddenColumn(key: string) {
    const next = new Set(hiddenColumns);
    if (next.has(key)) {
      next.delete(key);
    } else {
      next.add(key);
    }
    void setHiddenColumnsRaw(JSON.stringify(Array.from(next)));
  }

  return (
    <div>
      {/* Top bar */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          flexWrap: 'wrap',
          gap: 10,
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
            {t('holdings.title')}
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
            {t(holdings.length === 1 ? 'common.positions' : 'common.positions_plural', {
              count: holdings.length,
            })}
          </span>
        </div>
        <div
          style={{
            display: 'flex',
            gap: 10,
            alignItems: 'center',
            flexWrap: 'wrap',
            // Stretch to fill remaining space so wrapped lines keep right alignment.
            flex: 1,
            justifyContent: 'flex-end',
            minWidth: 0,
          }}
        >
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
          <Select
            value={accountFilter}
            onChange={(value) => handleAccountFilterChange(value as 'all' | AccountType)}
            options={[
              { value: 'all', label: t('holdings.allAccounts') },
              ...ACCOUNT_OPTIONS.map((option) => ({ value: option.value, label: option.label })),
            ]}
            style={{ width: 160 }}
          />
          {/* Column visibility dropdown */}
          <div ref={colPickerRef} style={{ position: 'relative' }}>
            <button
              onClick={() => setColPickerOpen((v) => !v)}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 4,
                padding: '6px 10px',
                background: colPickerOpen ? 'rgba(59,130,246,0.12)' : 'var(--bg-surface)',
                border: colPickerOpen
                  ? '1px solid var(--color-accent)'
                  : '1px solid var(--border-primary)',
                color: colPickerOpen ? 'var(--color-accent)' : 'var(--text-muted)',
                borderRadius: '2px',
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                cursor: 'pointer',
              }}
            >
              Columns
            </button>
            {colPickerOpen && (
              <div
                style={{
                  position: 'absolute',
                  top: '100%',
                  right: 0,
                  marginTop: 4,
                  zIndex: 200,
                  background: 'var(--bg-surface)',
                  border: '1px solid var(--border-primary)',
                  borderRadius: 2,
                  maxHeight: 360,
                  overflowY: 'auto',
                  minWidth: 160,
                }}
              >
                {ALL_COLUMNS.map((col) => {
                  const isAlways = ALWAYS_VISIBLE_COLS.has(col.key);
                  const isVisible = !hiddenColumns.has(col.key);
                  return (
                    <label
                      key={col.key}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: 8,
                        padding: '6px 12px',
                        cursor: isAlways ? 'not-allowed' : 'pointer',
                        opacity: isAlways ? 0.45 : 1,
                        fontFamily: 'var(--font-mono)',
                        fontSize: 11,
                        color: 'var(--text-primary)',
                        userSelect: 'none',
                      }}
                      onMouseEnter={(e) => {
                        if (!isAlways)
                          (e.currentTarget as HTMLElement).style.background =
                            'var(--bg-surface-hover)';
                      }}
                      onMouseLeave={(e) => {
                        (e.currentTarget as HTMLElement).style.background = 'transparent';
                      }}
                    >
                      <input
                        type="checkbox"
                        checked={isVisible}
                        disabled={isAlways}
                        onChange={() => {
                          if (!isAlways) toggleHiddenColumn(col.key);
                        }}
                        style={{
                          accentColor: 'var(--color-accent)',
                          cursor: isAlways ? 'not-allowed' : 'pointer',
                        }}
                      />
                      {col.label}
                    </label>
                  );
                })}
              </div>
            )}
          </div>
          <button
            onClick={() => setGroupByAccount((prev) => !prev)}
            title={groupByAccount ? 'Disable group by account' : 'Group by account'}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '6px 12px',
              background: groupByAccount ? 'var(--color-accent)' : 'var(--bg-surface)',
              border: groupByAccount
                ? '1px solid var(--color-accent)'
                : '1px solid var(--border-primary)',
              color: groupByAccount ? '#fff' : 'var(--text-primary)',
              borderRadius: '2px',
              fontFamily: 'var(--font-mono)',
              fontSize: 11,
              cursor: 'pointer',
            }}
          >
            <Layers size={12} />
            Group
          </button>
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
            {t('holdings.exportCsv')}
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
            {t('holdings.importCsv')}
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
            {t('holdings.addHolding')}
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

          {/* Grouped-by-account view */}
          {groupByAccount && groupedRows && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              {groupedRows.map((group) => {
                const isCollapsed = collapsedGroups.has(group.account);
                const acctConfig = ACCOUNT_TYPE_CONFIG[group.account];
                const acctColor = acctConfig?.color ?? 'var(--text-muted)';
                const acctLabel = acctConfig?.label ?? group.account;
                return (
                  <div key={group.account} style={{ border: '1px solid var(--border-primary)' }}>
                    {/* Section header */}
                    <button
                      onClick={() => toggleGroup(group.account)}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: 10,
                        width: '100%',
                        padding: '10px 12px',
                        background: 'var(--bg-surface-alt)',
                        border: 'none',
                        borderBottom: isCollapsed ? 'none' : '1px solid var(--border-primary)',
                        cursor: 'pointer',
                        textAlign: 'left',
                      }}
                    >
                      {isCollapsed ? (
                        <ChevronDown
                          size={13}
                          style={{ color: 'var(--text-muted)', flexShrink: 0 }}
                        />
                      ) : (
                        <ChevronUp
                          size={13}
                          style={{ color: 'var(--text-muted)', flexShrink: 0 }}
                        />
                      )}
                      <span
                        style={{
                          fontSize: 10,
                          fontFamily: 'var(--font-mono)',
                          fontWeight: 700,
                          textTransform: 'uppercase',
                          letterSpacing: '0.08em',
                          padding: '2px 6px',
                          borderRadius: 2,
                          background: `${acctColor}22`,
                          color: acctColor,
                          border: `1px solid ${acctColor}55`,
                        }}
                      >
                        {acctLabel}
                      </span>
                      <span
                        style={{
                          fontFamily: 'var(--font-mono)',
                          fontSize: 11,
                          color: 'var(--text-secondary)',
                          flex: 1,
                        }}
                      >
                        {group.holdings.length} holding{group.holdings.length !== 1 ? 's' : ''}
                      </span>
                      <span
                        style={{
                          fontFamily: 'var(--font-mono)',
                          fontSize: 12,
                          fontWeight: 600,
                          color: 'var(--text-primary)',
                        }}
                      >
                        {formatCurrency(group.totalValue, baseCurrency)}
                      </span>
                      <span
                        style={{
                          fontFamily: 'var(--font-mono)',
                          fontSize: 11,
                          color: pnlColor(group.totalGainLoss),
                          minWidth: 60,
                          textAlign: 'right',
                        }}
                      >
                        {group.totalGainLoss >= 0 ? '+' : ''}
                        {formatCurrency(group.totalGainLoss, baseCurrency)}
                      </span>
                      <span
                        style={{
                          fontFamily: 'var(--font-mono)',
                          fontSize: 11,
                          color: 'var(--text-muted)',
                          minWidth: 44,
                          textAlign: 'right',
                        }}
                      >
                        {group.totalWeight.toFixed(1)}%
                      </span>
                    </button>
                    {/* Holdings rows */}
                    {!isCollapsed && (
                      <div style={{ overflowX: 'auto' }}>
                        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 12 }}>
                          <thead>
                            <tr>
                              {columns.map(({ key, label, align }) => (
                                <th
                                  key={key}
                                  onClick={() => toggleSort(key)}
                                  style={{ ...TH, textAlign: align }}
                                >
                                  <span
                                    style={{
                                      display: 'inline-flex',
                                      alignItems: 'center',
                                      gap: 3,
                                    }}
                                  >
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
                              <th style={{ ...TH, textAlign: 'center' }}>
                                {t('holdings.columns.actions')}
                              </th>
                            </tr>
                          </thead>
                          <tbody>
                            {group.holdings.map((h, i) => {
                              const isDelGrp = deletingId === h.id;
                              const isPendGrp = pendingDelete === h.id;
                              const bgGrp =
                                i % 2 === 0 ? 'var(--bg-surface)' : 'var(--bg-surface-alt)';
                              return (
                                <tr
                                  key={h.id}
                                  style={{
                                    background: isPendGrp
                                      ? 'rgba(255,71,87,0.08)'
                                      : isDelGrp
                                        ? 'rgba(255,71,87,0.15)'
                                        : bgGrp,
                                    transition: 'background 200ms',
                                  }}
                                  onMouseEnter={(e) => {
                                    if (!isPendGrp && !isDelGrp)
                                      (e.currentTarget as HTMLElement).style.background =
                                        'var(--bg-surface-hover)';
                                  }}
                                  onMouseLeave={(e) => {
                                    if (!isPendGrp && !isDelGrp)
                                      (e.currentTarget as HTMLElement).style.background = bgGrp;
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
                                      maxWidth: 140,
                                      overflow: 'hidden',
                                      textOverflow: 'ellipsis',
                                    }}
                                  >
                                    {h.name}
                                  </td>
                                  <td style={TD}>
                                    <Badge type={h.assetType} />
                                  </td>
                                  <td style={TD}>
                                    <AccountBadge account={h.account} />
                                  </td>
                                  <td style={TD}>
                                    {h.exchange ? (
                                      <ExchangeBadge exchange={h.exchange} />
                                    ) : (
                                      <span style={{ color: 'var(--text-muted)' }}>—</span>
                                    )}
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
                                    {h.assetType === 'cash' ? (
                                      '—'
                                    ) : (
                                      <span
                                        style={{
                                          display: 'inline-flex',
                                          alignItems: 'center',
                                          gap: 4,
                                        }}
                                      >
                                        {formatNumber(h.currentPrice, 2)} {h.currency}
                                        {isPriceStale(portfolio?.lastUpdated) && (
                                          <span title="Price may be stale (last refreshed over 2h ago)">
                                            <Clock
                                              size={10}
                                              style={{
                                                color: 'var(--color-warning)',
                                                flexShrink: 0,
                                              }}
                                            />
                                          </span>
                                        )}
                                      </span>
                                    )}
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
                                      color:
                                        h.targetWeight > 0
                                          ? 'var(--text-primary)'
                                          : 'var(--text-muted)',
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
                                        h.assetType === 'cash'
                                          ? 'var(--text-muted)'
                                          : pnlColor(h.gainLoss),
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
                                    {h.assetType === 'cash'
                                      ? '—'
                                      : formatPercent(h.gainLossPercent)}
                                  </td>
                                  <td style={{ ...TD, textAlign: 'center', borderRight: 'none' }}>
                                    {isPendGrp ? (
                                      <span
                                        style={{
                                          display: 'inline-flex',
                                          gap: 6,
                                          alignItems: 'center',
                                        }}
                                      >
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
                                      <span
                                        style={{
                                          display: 'inline-flex',
                                          gap: 8,
                                          alignItems: 'center',
                                        }}
                                      >
                                        <button
                                          onClick={() => setTxModalHolding(h)}
                                          title="Log transaction"
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
                                          <Plus size={13} />
                                        </button>
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
                                          disabled={isDeleting}
                                          style={{
                                            background: 'none',
                                            border: 'none',
                                            color: 'var(--text-muted)',
                                            cursor: isDeleting ? 'not-allowed' : 'pointer',
                                            padding: 3,
                                            display: 'flex',
                                            alignItems: 'center',
                                            opacity: isDeleting ? 0.4 : 1,
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
                          {/* Subtotal row */}
                          <tfoot>
                            <tr style={{ background: 'var(--bg-surface-alt)' }}>
                              <td
                                colSpan={8}
                                style={{
                                  ...TD,
                                  fontFamily: 'var(--font-mono)',
                                  fontWeight: 600,
                                  color: 'var(--text-muted)',
                                  fontSize: 10,
                                  textTransform: 'uppercase',
                                  letterSpacing: '0.06em',
                                }}
                              >
                                Subtotal
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
                                {formatCurrency(group.totalValue, baseCurrency)}
                              </td>
                              <td
                                style={{
                                  ...TD,
                                  textAlign: 'right',
                                  fontFamily: 'var(--font-mono)',
                                  color: 'var(--text-secondary)',
                                }}
                              >
                                {(group.totalWeight * 100).toFixed(2)}%
                              </td>
                              <td colSpan={3} style={TD} />
                              <td
                                style={{
                                  ...TD,
                                  textAlign: 'right',
                                  fontFamily: 'var(--font-mono)',
                                  fontWeight: 600,
                                  color: pnlColor(group.totalGainLoss),
                                }}
                              >
                                {group.totalGainLoss >= 0 ? '+' : ''}
                                {formatCurrency(group.totalGainLoss, baseCurrency)}
                              </td>
                              <td colSpan={2} style={TD} />
                            </tr>
                          </tfoot>
                        </table>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}

          {/* Bulk action bar (flat view only) */}
          {!groupByAccount && selected.size > 0 && (
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 10,
                padding: '8px 12px',
                background: 'var(--bg-surface)',
                border: '1px solid var(--border-primary)',
                borderBottom: 'none',
                marginBottom: 0,
              }}
            >
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 12,
                  color: 'var(--text-secondary)',
                  flex: 1,
                }}
              >
                {selected.size} selected
              </span>
              <button
                onClick={() => setBulkDeletePending(true)}
                disabled={bulkDeleting || isDeleting}
                style={{
                  padding: '4px 12px',
                  background: 'transparent',
                  border: '1px solid var(--color-loss)',
                  color: 'var(--color-loss)',
                  borderRadius: '2px',
                  fontFamily: 'var(--font-mono)',
                  fontSize: 11,
                  cursor: bulkDeleting || isDeleting ? 'not-allowed' : 'pointer',
                }}
              >
                Delete selected
              </button>
              <button
                onClick={() => setSelected(new Set())}
                disabled={bulkDeleting}
                style={{
                  padding: '4px 12px',
                  background: 'transparent',
                  border: '1px solid var(--border-primary)',
                  color: 'var(--text-secondary)',
                  borderRadius: '2px',
                  fontFamily: 'var(--font-mono)',
                  fontSize: 11,
                  cursor: bulkDeleting ? 'not-allowed' : 'pointer',
                }}
              >
                Clear selection
              </button>
            </div>
          )}

          {/* Bulk delete confirmation (flat view only) */}
          {!groupByAccount && bulkDeletePending && (
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 10,
                padding: '8px 12px',
                background: 'rgba(255,71,87,0.08)',
                border: '1px solid var(--color-loss)',
                borderBottom: 'none',
              }}
            >
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 12,
                  color: 'var(--color-loss)',
                  flex: 1,
                }}
              >
                Delete {selected.size} holding{selected.size !== 1 ? 's' : ''}? This cannot be
                undone.
              </span>
              <button
                onClick={() => void handleBulkDelete()}
                disabled={bulkDeleting}
                style={{
                  padding: '4px 12px',
                  background: 'var(--color-loss)',
                  border: 'none',
                  color: '#fff',
                  borderRadius: '2px',
                  fontFamily: 'var(--font-mono)',
                  fontSize: 11,
                  fontWeight: 600,
                  cursor: bulkDeleting ? 'not-allowed' : 'pointer',
                }}
              >
                {bulkDeleting ? 'Deleting...' : 'Confirm'}
              </button>
              <button
                onClick={() => setBulkDeletePending(false)}
                disabled={bulkDeleting}
                style={{
                  padding: '4px 12px',
                  background: 'transparent',
                  border: '1px solid var(--border-primary)',
                  color: 'var(--text-secondary)',
                  borderRadius: '2px',
                  fontFamily: 'var(--font-mono)',
                  fontSize: 11,
                  cursor: bulkDeleting ? 'not-allowed' : 'pointer',
                }}
              >
                Cancel
              </button>
            </div>
          )}

          {/* Flat table view */}
          {!groupByAccount && (
            <div
              style={{
                border: '1px solid var(--border-primary)',
                overflowX: 'auto',
                overflowY: 'auto',
                maxHeight: 'calc(100vh - 260px)',
                width: '100%',
              }}
            >
              <table
                style={{ minWidth: 900, width: '100%', borderCollapse: 'collapse', fontSize: 12 }}
              >
                <thead style={{ position: 'sticky', top: 0, zIndex: 2 }}>
                  <tr>
                    <th style={{ ...TH, textAlign: 'center', width: 36, cursor: 'default' }}>
                      <input
                        type="checkbox"
                        checked={selected.size === rows.length && rows.length > 0}
                        onChange={toggleSelectAll}
                        title="Select all"
                        style={{ accentColor: 'var(--color-accent)', cursor: 'pointer' }}
                      />
                    </th>
                    {columns
                      .filter(({ key }) => !hiddenColumns.has(key))
                      .map(({ key, label, align }) => (
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
                    {!hiddenColumns.has('prevClose') && (
                      <th style={{ ...TH, textAlign: 'right' }}>Prev Close</th>
                    )}
                    {!hiddenColumns.has('dayOpen') && (
                      <th style={{ ...TH, textAlign: 'right' }}>Day Open</th>
                    )}
                    {!hiddenColumns.has('openDate') && (
                      <th style={{ ...TH, textAlign: 'right' }}>Open Date</th>
                    )}
                    {!hiddenColumns.has('maturityDate') && (
                      <th style={{ ...TH, textAlign: 'right' }}>Maturity</th>
                    )}
                    <th style={{ ...TH, textAlign: 'center' }}>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((h, i) => {
                    const isRowDeleting = deletingId === h.id;
                    const isPending = pendingDelete === h.id;
                    const bg = i % 2 === 0 ? 'var(--bg-surface)' : 'var(--bg-surface-alt)';
                    return (
                      <tr
                        key={h.id}
                        style={{
                          background: isPending
                            ? 'rgba(255,71,87,0.08)'
                            : isRowDeleting
                              ? 'rgba(255,71,87,0.15)'
                              : selected.has(h.id)
                                ? 'rgba(59,130,246,0.08)'
                                : bg,
                          transition: 'background 200ms',
                        }}
                        onMouseEnter={(e) => {
                          if (!isPending && !isRowDeleting && !selected.has(h.id))
                            (e.currentTarget as HTMLElement).style.background =
                              'var(--bg-surface-hover)';
                        }}
                        onMouseLeave={(e) => {
                          if (!isPending && !isRowDeleting)
                            (e.currentTarget as HTMLElement).style.background = selected.has(h.id)
                              ? 'rgba(59,130,246,0.08)'
                              : bg;
                        }}
                      >
                        <td
                          style={{ ...TD, textAlign: 'center', width: 36 }}
                          onClick={(e) => e.stopPropagation()}
                        >
                          <input
                            type="checkbox"
                            checked={selected.has(h.id)}
                            onChange={() => toggleRow(h.id)}
                            style={{ accentColor: 'var(--color-accent)', cursor: 'pointer' }}
                          />
                        </td>
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
                        {!hiddenColumns.has('name') && (
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
                        )}
                        {!hiddenColumns.has('assetType') && (
                          <td style={TD}>
                            <Badge type={h.assetType} />
                          </td>
                        )}
                        {!hiddenColumns.has('account') && (
                          <td style={TD}>
                            <AccountBadge account={h.account} />
                          </td>
                        )}
                        {!hiddenColumns.has('exchange') && (
                          <td style={TD}>
                            {h.exchange ? (
                              <ExchangeBadge exchange={h.exchange} />
                            ) : (
                              <span style={{ color: 'var(--text-muted)' }}>—</span>
                            )}
                          </td>
                        )}
                        {!hiddenColumns.has('quantity') && (
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
                        )}
                        {!hiddenColumns.has('costBasis') && (
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
                        )}
                        {!hiddenColumns.has('currentPrice') && (
                          <td
                            style={{
                              ...TD,
                              textAlign: 'right',
                              fontFamily: 'var(--font-mono)',
                              color: 'var(--text-primary)',
                            }}
                          >
                            {h.assetType === 'cash' ? (
                              '—'
                            ) : (
                              <span
                                style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}
                              >
                                {formatNumber(h.currentPrice, 2)} {h.currency}
                                {isPriceStale(portfolio?.lastUpdated) && (
                                  <span title="Price may be stale (last refreshed over 2h ago)">
                                    <Clock
                                      size={10}
                                      style={{ color: 'var(--color-warning)', flexShrink: 0 }}
                                    />
                                  </span>
                                )}
                              </span>
                            )}
                          </td>
                        )}
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
                        {!hiddenColumns.has('weight') && (
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
                        )}
                        {!hiddenColumns.has('targetWeight') && (
                          <td
                            style={{
                              ...TD,
                              textAlign: 'right',
                              fontFamily: 'var(--font-mono)',
                              color:
                                h.targetWeight > 0 ? 'var(--text-primary)' : 'var(--text-muted)',
                            }}
                          >
                            {h.targetWeight > 0 ? `${h.targetWeight.toFixed(1)}%` : '—'}
                          </td>
                        )}
                        {!hiddenColumns.has('targetDeltaPercent') && (
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
                        )}
                        {!hiddenColumns.has('targetDeltaValue') && (
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
                        )}
                        {!hiddenColumns.has('gainLoss') && (
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
                        )}
                        {!hiddenColumns.has('gainLossPercent') && (
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
                        )}
                        {!hiddenColumns.has('prevClose') && (
                          <td
                            style={{
                              ...TD,
                              textAlign: 'right',
                              fontFamily: 'var(--font-mono)',
                              color: 'var(--text-secondary)',
                            }}
                          >
                            {h.assetType === 'cash'
                              ? '—'
                              : (() => {
                                  const pd = priceMap[h.symbol];
                                  return pd?.previousClose != null
                                    ? `${formatNumber(pd.previousClose, 2)} ${h.currency}`
                                    : '—';
                                })()}
                          </td>
                        )}
                        {!hiddenColumns.has('dayOpen') && (
                          <td
                            style={{
                              ...TD,
                              textAlign: 'right',
                              fontFamily: 'var(--font-mono)',
                              color: 'var(--text-secondary)',
                            }}
                          >
                            {h.assetType === 'cash'
                              ? '—'
                              : (() => {
                                  const pd = priceMap[h.symbol];
                                  return pd?.open != null
                                    ? `${formatNumber(pd.open, 2)} ${h.currency}`
                                    : '—';
                                })()}
                          </td>
                        )}
                        {!hiddenColumns.has('openDate') && (
                          <td
                            style={{
                              ...TD,
                              textAlign: 'right',
                              fontFamily: 'var(--font-mono)',
                              color: 'var(--text-secondary)',
                            }}
                          >
                            {formatShortDate(h.createdAt)}
                          </td>
                        )}
                        {!hiddenColumns.has('maturityDate') &&
                          (() => {
                            const md = h.maturityDate;
                            if (!md) return <td style={TD} />;
                            const daysUntil = Math.ceil(
                              (new Date(md).getTime() - Date.now()) / (1000 * 60 * 60 * 24)
                            );
                            const color =
                              daysUntil <= 0
                                ? 'var(--color-loss)'
                                : daysUntil <= 90
                                  ? 'var(--color-warning)'
                                  : 'var(--text-secondary)';
                            return (
                              <td
                                style={{
                                  ...TD,
                                  textAlign: 'right',
                                  fontFamily: 'var(--font-mono)',
                                  color,
                                }}
                              >
                                {formatMonthYear(md)}
                                {daysUntil <= 90 && daysUntil > 0 && (
                                  <span
                                    style={{
                                      marginLeft: 4,
                                      fontSize: 10,
                                      background: 'rgba(251,191,36,0.15)',
                                      color: 'var(--color-warning)',
                                      padding: '1px 4px',
                                      borderRadius: 2,
                                    }}
                                  >
                                    {daysUntil}d
                                  </span>
                                )}
                                {daysUntil <= 0 && (
                                  <span
                                    style={{
                                      marginLeft: 4,
                                      fontSize: 10,
                                      background: 'rgba(255,71,87,0.15)',
                                      color: 'var(--color-loss)',
                                      padding: '1px 4px',
                                      borderRadius: 2,
                                    }}
                                  >
                                    MATURED
                                  </span>
                                )}
                              </td>
                            );
                          })()}
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
                                onClick={() => setTxModalHolding(h)}
                                title="Log transaction"
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
                                <Plus size={13} />
                              </button>
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
                                disabled={isDeleting}
                                style={{
                                  background: 'none',
                                  border: 'none',
                                  color: isDeleting ? 'var(--text-muted)' : 'var(--text-muted)',
                                  cursor: isDeleting ? 'not-allowed' : 'pointer',
                                  padding: 3,
                                  display: 'flex',
                                  alignItems: 'center',
                                  opacity: isDeleting ? 0.4 : 1,
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
                    <td style={TD} />
                    <td
                      colSpan={8}
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
                        color:
                          totals.targetWeight > 0 ? 'var(--text-primary)' : 'var(--text-muted)',
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
          )}
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
      {txModalHolding && (
        <AddTransactionModal
          holding={txModalHolding}
          isOpen={txModalHolding !== undefined}
          onClose={() => setTxModalHolding(undefined)}
          onSaved={() => setTxModalHolding(undefined)}
        />
      )}
    </div>
  );
}
