import { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Plus, Trash2 } from 'lucide-react';
import { usePortfolio } from '../hooks/usePortfolio';
import { AddTransactionModal } from './AddTransactionModal';
import { EmptyState } from './ui/EmptyState';
import { Spinner } from './ui/Spinner';
import { useToast } from './ui/Toast';
import { formatNumber } from '../lib/format';
import type { Transaction, Holding } from '../types/portfolio';

type TxType = 'buy' | 'sell' | 'deposit' | 'withdrawal' | 'all';

const TX_TYPE_COLORS: Record<string, string> = {
  buy: 'var(--color-gain)',
  sell: 'var(--color-loss)',
  deposit: 'var(--color-accent)',
  withdrawal: 'var(--text-muted)',
};

function TxBadge({ type }: { type: string }) {
  const color = TX_TYPE_COLORS[type] ?? 'var(--text-muted)';
  return (
    <span
      style={{
        display: 'inline-block',
        padding: '1px 6px',
        borderRadius: '2px',
        border: `1px solid ${color}`,
        color,
        fontFamily: 'var(--font-mono)',
        fontSize: 10,
        textTransform: 'uppercase' as const,
        letterSpacing: '0.06em',
        fontWeight: 600,
        whiteSpace: 'nowrap' as const,
      }}
    >
      {type}
    </span>
  );
}

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return iso;
  }
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
};

const TD: React.CSSProperties = {
  padding: '7px 10px',
  fontSize: 12,
  fontFamily: 'var(--font-sans)',
  borderBottom: '1px solid var(--border-subtle)',
  color: 'var(--text-primary)',
  verticalAlign: 'middle',
};

export function TransactionHistory() {
  const { portfolio } = usePortfolio();
  const { showToast } = useToast();

  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<number | null>(null);
  const [pendingDelete, setPendingDelete] = useState<number | null>(null);

  const [filterHoldingId, setFilterHoldingId] = useState<string>('all');
  const [filterType, setFilterType] = useState<TxType>('all');

  const [modalOpen, setModalOpen] = useState(false);
  const [modalHolding, setModalHolding] = useState<Holding | null>(null);
  const [selectorHoldingId, setSelectorHoldingId] = useState<string>('');

  const holdings: Holding[] = useMemo(() => (portfolio?.holdings ?? []) as Holding[], [portfolio]);

  async function loadTransactions() {
    setLoading(true);
    setError(null);
    try {
      const txs = await invoke<Transaction[]>('get_transactions', {});
      setTransactions(txs);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadTransactions();
  }, []);

  // Set default selector holding when holdings are available
  useEffect(() => {
    if (holdings.length > 0 && !selectorHoldingId) {
      setSelectorHoldingId(holdings[0].id);
    }
  }, [holdings, selectorHoldingId]);

  const holdingById = useMemo(() => {
    const map = new Map<string, Holding>();
    for (const h of holdings) {
      map.set(h.id, h);
    }
    return map;
  }, [holdings]);

  const filteredTransactions = useMemo(() => {
    return transactions.filter((tx) => {
      if (filterHoldingId !== 'all' && tx.holdingId !== filterHoldingId) return false;
      if (filterType !== 'all' && tx.transactionType !== filterType) return false;
      return true;
    });
  }, [transactions, filterHoldingId, filterType]);

  // Group by holdingId for display
  const groupedByHolding = useMemo(() => {
    const groups = new Map<string, Transaction[]>();
    for (const tx of filteredTransactions) {
      const existing = groups.get(tx.holdingId) ?? [];
      groups.set(tx.holdingId, [...existing, tx]);
    }
    return groups;
  }, [filteredTransactions]);

  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  function toggleCollapse(holdingId: string) {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(holdingId)) {
        next.delete(holdingId);
      } else {
        next.add(holdingId);
      }
      return next;
    });
  }

  async function handleDelete(id: number) {
    setDeletingId(id);
    try {
      await invoke('delete_transaction', { id });
      setTransactions((prev) => prev.filter((tx) => tx.id !== id));
      showToast('Transaction deleted', 'info');
    } catch (err) {
      showToast(String(err), 'error');
    } finally {
      setDeletingId(null);
      setPendingDelete(null);
    }
  }

  function openAddModal() {
    const holding = selectorHoldingId ? holdingById.get(selectorHoldingId) : undefined;
    if (!holding) {
      showToast('Select a holding first', 'error');
      return;
    }
    setModalHolding(holding);
    setModalOpen(true);
  }

  const selectStyle: React.CSSProperties = {
    background: 'var(--bg-surface)',
    border: '1px solid var(--border-primary)',
    color: 'var(--text-primary)',
    padding: '6px 10px',
    fontSize: 11,
    fontFamily: 'var(--font-mono)',
    borderRadius: '2px',
  };

  return (
    <div>
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
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
            Transactions
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
            {filteredTransactions.length} records
          </span>
        </div>

        <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
          {/* Filter by holding */}
          <select
            value={filterHoldingId}
            onChange={(e) => setFilterHoldingId(e.target.value)}
            style={selectStyle}
          >
            <option value="all">All Holdings</option>
            {holdings.map((h) => (
              <option key={h.id} value={h.id}>
                {h.symbol} — {h.name}
              </option>
            ))}
          </select>

          {/* Filter by type */}
          <select
            value={filterType}
            onChange={(e) => setFilterType(e.target.value as TxType)}
            style={selectStyle}
          >
            <option value="all">All Types</option>
            <option value="buy">Buy</option>
            <option value="sell">Sell</option>
            <option value="deposit">Deposit</option>
            <option value="withdrawal">Withdrawal</option>
          </select>

          {/* Holding selector for add */}
          {holdings.length > 0 && (
            <select
              value={selectorHoldingId}
              onChange={(e) => setSelectorHoldingId(e.target.value)}
              style={selectStyle}
              title="Select holding for new transaction"
            >
              {holdings.map((h) => (
                <option key={h.id} value={h.id}>
                  {h.symbol}
                </option>
              ))}
            </select>
          )}

          <button
            onClick={openAddModal}
            disabled={holdings.length === 0}
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
              cursor: holdings.length === 0 ? 'not-allowed' : 'pointer',
              opacity: holdings.length === 0 ? 0.5 : 1,
            }}
          >
            <Plus size={13} />
            Add Transaction
          </button>
        </div>
      </div>

      {/* Content */}
      {loading ? (
        <div style={{ display: 'flex', justifyContent: 'center', padding: 40 }}>
          <Spinner />
        </div>
      ) : error ? (
        <div
          style={{
            padding: '16px',
            color: 'var(--color-loss)',
            fontFamily: 'var(--font-mono)',
            fontSize: 13,
            border: '1px solid var(--border-primary)',
          }}
        >
          Failed to load transactions: {error}
        </div>
      ) : filteredTransactions.length === 0 ? (
        <EmptyState
          message="No transactions recorded yet."
          action={
            holdings.length > 0 ? { label: '+ Add Transaction', onClick: openAddModal } : undefined
          }
        />
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          {Array.from(groupedByHolding.entries()).map(([holdingId, txs]) => {
            const holding = holdingById.get(holdingId);
            const isCollapsed = collapsed.has(holdingId);

            return (
              <div
                key={holdingId}
                style={{
                  border: '1px solid var(--border-primary)',
                }}
              >
                {/* Section header */}
                <button
                  onClick={() => toggleCollapse(holdingId)}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 10,
                    width: '100%',
                    padding: '10px 14px',
                    background: 'var(--bg-surface-alt)',
                    border: 'none',
                    borderBottom: isCollapsed ? 'none' : '1px solid var(--border-primary)',
                    color: 'var(--text-primary)',
                    cursor: 'pointer',
                    textAlign: 'left',
                  }}
                >
                  <span
                    style={{
                      fontFamily: 'var(--font-mono)',
                      fontWeight: 700,
                      fontSize: 13,
                      color: 'var(--text-primary)',
                    }}
                  >
                    {holding?.symbol ?? holdingId}
                  </span>
                  {holding && (
                    <span
                      style={{
                        fontFamily: 'var(--font-sans)',
                        fontSize: 11,
                        color: 'var(--text-secondary)',
                      }}
                    >
                      {holding.name}
                    </span>
                  )}
                  <span
                    style={{
                      fontFamily: 'var(--font-mono)',
                      fontSize: 10,
                      color: 'var(--text-muted)',
                      marginLeft: 'auto',
                    }}
                  >
                    {txs.length} {txs.length === 1 ? 'transaction' : 'transactions'}{' '}
                    {isCollapsed ? '▶' : '▼'}
                  </span>
                </button>

                {!isCollapsed && (
                  <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 12 }}>
                    <thead>
                      <tr>
                        <th style={{ ...TH, textAlign: 'left' }}>Date</th>
                        <th style={{ ...TH, textAlign: 'left' }}>Type</th>
                        <th style={{ ...TH, textAlign: 'right' }}>Quantity</th>
                        <th style={{ ...TH, textAlign: 'right' }}>Price</th>
                        <th style={{ ...TH, textAlign: 'right' }}>Fee</th>
                        <th style={{ ...TH, textAlign: 'right' }}>Total Value</th>
                        <th style={{ ...TH, textAlign: 'center', width: 60 }}>Actions</th>
                      </tr>
                    </thead>
                    <tbody>
                      {txs.map((tx, i) => {
                        const isDeleting = deletingId === tx.id;
                        const isPending = pendingDelete === tx.id;
                        const totalValue = tx.quantity * tx.price;
                        const bg = i % 2 === 0 ? 'var(--bg-surface)' : 'var(--bg-surface-alt)';
                        return (
                          <tr
                            key={tx.id}
                            style={{
                              background: isDeleting
                                ? 'rgba(255,71,87,0.15)'
                                : isPending
                                  ? 'rgba(255,71,87,0.08)'
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
                            <td style={{ ...TD, color: 'var(--text-secondary)' }}>
                              {formatDate(tx.transactedAt)}
                            </td>
                            <td style={TD}>
                              <TxBadge type={tx.transactionType} />
                            </td>
                            <td
                              style={{
                                ...TD,
                                textAlign: 'right',
                                fontFamily: 'var(--font-mono)',
                              }}
                            >
                              {formatNumber(tx.quantity, 4)}
                            </td>
                            <td
                              style={{
                                ...TD,
                                textAlign: 'right',
                                fontFamily: 'var(--font-mono)',
                              }}
                            >
                              {formatNumber(tx.price, 2)} {tx.currency}
                            </td>
                            <td
                              style={{
                                ...TD,
                                textAlign: 'right',
                                fontFamily: 'var(--font-mono)',
                                color: tx.fee > 0 ? 'var(--color-loss)' : 'var(--text-muted)',
                              }}
                            >
                              {tx.fee > 0 ? formatNumber(tx.fee, 2) : '—'}
                            </td>
                            <td
                              style={{
                                ...TD,
                                textAlign: 'right',
                                fontFamily: 'var(--font-mono)',
                                fontWeight: 600,
                              }}
                            >
                              {formatNumber(totalValue, 2)} {tx.currency}
                            </td>
                            <td style={{ ...TD, textAlign: 'center', borderRight: 'none' }}>
                              {isPending ? (
                                <span
                                  style={{
                                    display: 'inline-flex',
                                    gap: 4,
                                    alignItems: 'center',
                                  }}
                                >
                                  <button
                                    onClick={() => void handleDelete(tx.id)}
                                    style={{
                                      fontSize: 10,
                                      color: 'var(--color-loss)',
                                      background: 'none',
                                      border: '1px solid var(--color-loss)',
                                      padding: '2px 5px',
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
                                      fontSize: 10,
                                      color: 'var(--text-secondary)',
                                      background: 'none',
                                      border: '1px solid var(--border-primary)',
                                      padding: '2px 5px',
                                      borderRadius: '2px',
                                      cursor: 'pointer',
                                      fontFamily: 'var(--font-mono)',
                                    }}
                                  >
                                    Cancel
                                  </button>
                                </span>
                              ) : (
                                <button
                                  onClick={() => setPendingDelete(tx.id)}
                                  title="Delete"
                                  disabled={isDeleting}
                                  style={{
                                    background: 'none',
                                    border: 'none',
                                    color: 'var(--text-muted)',
                                    cursor: 'pointer',
                                    padding: 3,
                                    display: 'inline-flex',
                                    alignItems: 'center',
                                  }}
                                >
                                  <Trash2 size={13} />
                                </button>
                              )}
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* Add Transaction Modal */}
      {modalHolding && (
        <AddTransactionModal
          holding={modalHolding}
          isOpen={modalOpen}
          onClose={() => setModalOpen(false)}
          onSaved={() => void loadTransactions()}
        />
      )}
    </div>
  );
}
