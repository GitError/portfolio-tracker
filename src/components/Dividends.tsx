import { useCallback, useEffect, useMemo, useState } from 'react';
import { DollarSign, Plus, Trash2 } from 'lucide-react';
import type { Dividend, DividendInput, Holding } from '../types/portfolio';
import { formatCurrency } from '../lib/format';
import { EmptyState } from './ui/EmptyState';
import { Spinner } from './ui/Spinner';
import { useToast } from './ui/Toast';
import { isTauri, tauriInvoke } from '../lib/tauri';

const MOCK_DIVIDENDS: Dividend[] = [
  {
    id: 1,
    holdingId: 'h1',
    symbol: 'VDY.TO',
    amountPerUnit: 0.118,
    currency: 'CAD',
    exDate: '2026-01-15',
    payDate: '2026-01-31',
    createdAt: '2026-01-01T00:00:00Z',
  },
  {
    id: 2,
    holdingId: 'h2',
    symbol: 'AAPL',
    amountPerUnit: 0.25,
    currency: 'USD',
    exDate: '2026-02-07',
    payDate: '2026-02-13',
    createdAt: '2026-01-20T00:00:00Z',
  },
];

const MOCK_HOLDINGS: Holding[] = [
  {
    id: 'h1',
    symbol: 'VDY.TO',
    name: 'Vanguard FTSE Canadian High Dividend',
    assetType: 'etf',
    account: 'tfsa',
    quantity: 100,
    costBasis: 42,
    currency: 'CAD',
    exchange: 'TSX',
    targetWeight: 0,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
  },
];

interface AddDividendFormProps {
  holdings: Holding[];
  onAdd: (input: DividendInput) => void;
  onCancel: () => void;
}

function AddDividendForm({ holdings, onAdd, onCancel }: AddDividendFormProps) {
  const [holdingId, setHoldingId] = useState(holdings[0]?.id ?? '');
  const [amount, setAmount] = useState('');
  const [currency, setCurrency] = useState('CAD');
  const [exDate, setExDate] = useState('');
  const [payDate, setPayDate] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const a = parseFloat(amount);
    if (!holdingId || isNaN(a) || a <= 0 || !exDate || !payDate) return;
    onAdd({ holdingId, amountPerUnit: a, currency, exDate, payDate });
  };

  const inputStyle: React.CSSProperties = {
    background: 'var(--bg-surface-alt)',
    border: '1px solid var(--border-primary)',
    color: 'var(--text-primary)',
    fontSize: 13,
    padding: '6px 10px',
    borderRadius: 2,
    outline: 'none',
    fontFamily: 'var(--font-sans)',
    width: '100%',
    boxSizing: 'border-box',
  };

  return (
    <form
      onSubmit={handleSubmit}
      style={{
        background: 'var(--bg-surface)',
        border: '1px solid var(--border-primary)',
        padding: 16,
        borderRadius: 2,
        marginBottom: 16,
        display: 'flex',
        flexDirection: 'column',
        gap: 12,
      }}
    >
      <div style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-primary)' }}>
        Record Dividend
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr 1fr', gap: 10 }}>
        <div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>HOLDING</div>
          <select
            style={inputStyle}
            value={holdingId}
            onChange={(e) => setHoldingId(e.target.value)}
            required
          >
            {holdings.map((h) => (
              <option key={h.id} value={h.id}>
                {h.symbol} — {h.name}
              </option>
            ))}
          </select>
        </div>
        <div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>
            AMOUNT / UNIT
          </div>
          <input
            style={{ ...inputStyle, fontFamily: 'var(--font-mono)' }}
            type="number"
            min="0"
            step="any"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="0.00"
            required
          />
        </div>
        <div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>CURRENCY</div>
          <select style={inputStyle} value={currency} onChange={(e) => setCurrency(e.target.value)}>
            {['CAD', 'USD', 'EUR', 'GBP'].map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>
        </div>
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
        <div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>EX-DATE</div>
          <input
            style={inputStyle}
            type="date"
            value={exDate}
            onChange={(e) => setExDate(e.target.value)}
            required
          />
        </div>
        <div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>PAY DATE</div>
          <input
            style={inputStyle}
            type="date"
            value={payDate}
            onChange={(e) => setPayDate(e.target.value)}
            required
          />
        </div>
      </div>
      <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
        <button
          type="button"
          onClick={onCancel}
          style={{
            background: 'transparent',
            border: '1px solid var(--border-primary)',
            color: 'var(--text-secondary)',
            fontSize: 13,
            padding: '6px 14px',
            borderRadius: 2,
            cursor: 'pointer',
          }}
        >
          Cancel
        </button>
        <button
          type="submit"
          style={{
            background: 'var(--color-accent)',
            border: 'none',
            color: '#fff',
            fontSize: 13,
            fontWeight: 500,
            padding: '6px 14px',
            borderRadius: 2,
            cursor: 'pointer',
          }}
        >
          Record
        </button>
      </div>
    </form>
  );
}

export function Dividends() {
  const [dividends, setDividends] = useState<Dividend[]>([]);
  const [holdings, setHoldings] = useState<Holding[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const { showToast } = useToast();

  const loadData = useCallback(async () => {
    try {
      if (isTauri()) {
        const [divs, holds] = await Promise.all([
          tauriInvoke<Dividend[]>('get_dividends'),
          tauriInvoke<Holding[]>('get_holdings'),
        ]);
        setDividends(divs);
        setHoldings(holds);
      } else {
        setDividends(MOCK_DIVIDENDS);
        setHoldings(MOCK_HOLDINGS);
      }
    } catch (err) {
      showToast(`Failed to load dividends: ${String(err)}`, 'error');
    } finally {
      setLoading(false);
    }
  }, [showToast]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  const handleAdd = useCallback(
    async (input: DividendInput) => {
      try {
        if (isTauri()) {
          const div = await tauriInvoke<Dividend>('add_dividend', { dividend: input });
          setDividends((prev) => [div, ...prev]);
        } else {
          const holding = holdings.find((h) => h.id === input.holdingId);
          const mock: Dividend = {
            id: Date.now(),
            symbol: holding?.symbol ?? '',
            ...input,
            createdAt: new Date().toISOString(),
          };
          setDividends((prev) => [mock, ...prev]);
        }
        setShowForm(false);
        showToast('Dividend recorded', 'success');
      } catch (err) {
        showToast(`Failed to record dividend: ${String(err)}`, 'error');
      }
    },
    [holdings, showToast]
  );

  const handleDelete = useCallback(
    async (id: number) => {
      try {
        if (isTauri()) {
          await tauriInvoke<boolean>('delete_dividend', { id });
        }
        setDividends((prev) => prev.filter((d) => d.id !== id));
        showToast('Dividend deleted', 'success');
      } catch (err) {
        showToast(`Failed to delete: ${String(err)}`, 'error');
      }
    },
    [showToast]
  );

  // Summary stats
  const summary = useMemo(() => {
    const bySymbol: Record<string, { total: number; currency: string; count: number }> = {};
    for (const div of dividends) {
      if (!bySymbol[div.symbol]) {
        bySymbol[div.symbol] = { total: 0, currency: div.currency, count: 0 };
      }
      bySymbol[div.symbol].total += div.amountPerUnit;
      bySymbol[div.symbol].count += 1;
    }
    return bySymbol;
  }, [dividends]);

  if (loading) {
    return (
      <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Spinner />
      </div>
    );
  }

  return (
    <div style={{ flex: 1, overflow: 'auto', padding: '24px 32px', maxWidth: 900 }}>
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          marginBottom: 24,
        }}
      >
        <div>
          <h1
            style={{
              fontSize: 18,
              fontWeight: 600,
              color: 'var(--text-primary)',
              margin: 0,
              marginBottom: 4,
            }}
          >
            Dividends
          </h1>
          <p style={{ fontSize: 13, color: 'var(--text-secondary)', margin: 0 }}>
            Record and track dividend income across your holdings.
          </p>
        </div>
        <button
          onClick={() => setShowForm(true)}
          disabled={holdings.length === 0}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            background: 'var(--color-accent)',
            border: 'none',
            color: '#fff',
            fontSize: 13,
            fontWeight: 500,
            padding: '8px 14px',
            borderRadius: 2,
            cursor: holdings.length === 0 ? 'not-allowed' : 'pointer',
            opacity: holdings.length === 0 ? 0.5 : 1,
          }}
        >
          <Plus size={14} />
          Record Dividend
        </button>
      </div>

      {/* Summary by symbol */}
      {Object.keys(summary).length > 0 && (
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fill, minmax(180px, 1fr))',
            gap: 1,
            background: 'var(--border-primary)',
            border: '1px solid var(--border-primary)',
            marginBottom: 24,
          }}
        >
          {Object.entries(summary).map(([symbol, data]) => (
            <div key={symbol} style={{ background: 'var(--bg-surface)', padding: '14px 16px' }}>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 13,
                  fontWeight: 600,
                  color: 'var(--text-primary)',
                }}
              >
                {symbol}
              </div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 16,
                  fontWeight: 600,
                  color: 'var(--color-gain)',
                  marginTop: 4,
                }}
              >
                {formatCurrency(data.total, data.currency)}
              </div>
              <div style={{ fontSize: 11, color: 'var(--text-muted)', marginTop: 2 }}>
                {data.count} payment{data.count !== 1 ? 's' : ''} recorded
              </div>
            </div>
          ))}
        </div>
      )}

      {showForm && (
        <AddDividendForm
          holdings={holdings}
          onAdd={handleAdd}
          onCancel={() => setShowForm(false)}
        />
      )}

      {/* Dividend history table */}
      {dividends.length === 0 ? (
        <EmptyState
          message={
            holdings.length === 0
              ? 'Add holdings first to record dividends.'
              : 'No dividends recorded yet. Click "Record Dividend" to add one.'
          }
        />
      ) : (
        <div
          style={{
            border: '1px solid var(--border-primary)',
            overflow: 'hidden',
          }}
        >
          {/* Header row */}
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: '120px 1fr 120px 110px 110px 48px',
              padding: '8px 16px',
              background: 'var(--bg-surface-alt)',
              borderBottom: '1px solid var(--border-primary)',
            }}
          >
            {['SYMBOL', 'EX-DATE', 'PAY DATE', 'AMT / UNIT', 'CURRENCY', ''].map((h) => (
              <div
                key={h}
                style={{
                  fontSize: 10,
                  fontWeight: 600,
                  textTransform: 'uppercase',
                  letterSpacing: '0.06em',
                  color: 'var(--text-muted)',
                }}
              >
                {h}
              </div>
            ))}
          </div>

          {dividends.map((div, i) => (
            <div
              key={div.id}
              style={{
                display: 'grid',
                gridTemplateColumns: '120px 1fr 120px 110px 110px 48px',
                padding: '10px 16px',
                alignItems: 'center',
                background: i % 2 === 0 ? 'var(--bg-surface)' : 'var(--bg-surface-alt)',
                borderBottom: i < dividends.length - 1 ? '1px solid var(--border-subtle)' : 'none',
              }}
            >
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 13,
                  fontWeight: 600,
                  color: 'var(--text-primary)',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 6,
                }}
              >
                <DollarSign size={12} style={{ color: 'var(--color-gain)', flexShrink: 0 }} />
                {div.symbol}
              </div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 12,
                  color: 'var(--text-primary)',
                }}
              >
                {div.exDate}
              </div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 12,
                  color: 'var(--text-secondary)',
                }}
              >
                {div.payDate}
              </div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 13,
                  fontWeight: 500,
                  color: 'var(--color-gain)',
                  textAlign: 'right',
                }}
              >
                +{div.amountPerUnit.toFixed(4)}
              </div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 12,
                  color: 'var(--text-secondary)',
                  textAlign: 'center',
                }}
              >
                {div.currency}
              </div>
              <div style={{ display: 'flex', justifyContent: 'center' }}>
                <button
                  onClick={() => handleDelete(div.id)}
                  title="Delete"
                  style={{
                    background: 'transparent',
                    border: '1px solid var(--border-primary)',
                    color: 'var(--text-secondary)',
                    cursor: 'pointer',
                    padding: '4px 6px',
                    borderRadius: 2,
                    display: 'flex',
                    alignItems: 'center',
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLButtonElement).style.color = 'var(--color-loss)';
                    (e.currentTarget as HTMLButtonElement).style.borderColor = 'var(--color-loss)';
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLButtonElement).style.color = 'var(--text-secondary)';
                    (e.currentTarget as HTMLButtonElement).style.borderColor =
                      'var(--border-primary)';
                  }}
                >
                  <Trash2 size={12} />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
