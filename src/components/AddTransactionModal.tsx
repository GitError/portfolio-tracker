import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Holding, Transaction, TransactionInput } from '../types/portfolio';

type TxType = 'buy' | 'sell';

interface Props {
  holding: Holding;
  isOpen: boolean;
  onClose: () => void;
  onSaved: () => void;
}

const TX_TYPES: { value: TxType; label: string }[] = [
  { value: 'buy', label: 'Buy' },
  { value: 'sell', label: 'Sell' },
];

function formatLocalDatetime(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  return (
    `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}` +
    `T${pad(date.getHours())}:${pad(date.getMinutes())}`
  );
}

export function AddTransactionModal({ holding, isOpen, onClose, onSaved }: Props) {
  const [txType, setTxType] = useState<TxType>('buy');
  const [quantity, setQuantity] = useState('');
  const [price, setPrice] = useState('');
  const [transactedAt, setTransactedAt] = useState(() => formatLocalDatetime(new Date()));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  function validate(): string | null {
    const qty = parseFloat(quantity);
    const prc = parseFloat(price);
    if (isNaN(qty) || qty <= 0) return 'Quantity must be greater than 0.';
    if (isNaN(prc) || prc < 0) return 'Price must be 0 or greater.';
    if (!transactedAt.trim()) return 'Date is required.';
    return null;
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const validationError = validate();
    if (validationError) {
      setError(validationError);
      return;
    }
    setError(null);
    setSaving(true);
    try {
      const input: TransactionInput = {
        holdingId: holding.id,
        transactionType: txType,
        quantity: parseFloat(quantity),
        price: parseFloat(price),
        transactedAt: new Date(transactedAt).toISOString(),
      };
      await invoke<Transaction>('add_transaction', { input });
      onSaved();
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }

  function handleClose() {
    setError(null);
    onClose();
  }

  const inputStyle: React.CSSProperties = {
    background: 'var(--bg-primary)',
    border: '1px solid var(--border-primary)',
    color: 'var(--text-primary)',
    padding: '7px 10px',
    fontSize: 12,
    fontFamily: 'var(--font-mono)',
    borderRadius: '2px',
    outline: 'none',
    width: '100%',
    boxSizing: 'border-box',
  };

  const labelStyle: React.CSSProperties = {
    fontSize: 11,
    color: 'var(--text-secondary)',
    fontFamily: 'var(--font-sans)',
    marginBottom: 4,
    display: 'block',
    textTransform: 'uppercase',
    letterSpacing: '0.06em',
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        background: 'rgba(0,0,0,0.55)',
        backdropFilter: 'blur(4px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 100,
      }}
      onClick={handleClose}
    >
      <div
        style={{
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-primary)',
          borderRadius: '2px',
          padding: '28px 28px 24px',
          width: '100%',
          maxWidth: 460,
          boxSizing: 'border-box',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div style={{ marginBottom: 20 }}>
          <div
            style={{
              fontFamily: 'var(--font-sans)',
              fontWeight: 600,
              fontSize: 15,
              color: 'var(--text-primary)',
            }}
          >
            Log Transaction
          </div>
          <div
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 11,
              color: 'var(--text-muted)',
              marginTop: 3,
            }}
          >
            {holding.symbol} &mdash; {holding.name}
          </div>
        </div>

        <form onSubmit={(e) => void handleSubmit(e)}>
          {/* Transaction type tabs */}
          <div style={{ marginBottom: 16 }}>
            <span style={labelStyle}>Type</span>
            <div style={{ display: 'flex', gap: 6 }}>
              {TX_TYPES.map(({ value, label }) => {
                const isActive = txType === value;
                const activeColor = value === 'buy' ? 'var(--color-gain)' : 'var(--color-loss)';
                return (
                  <button
                    key={value}
                    type="button"
                    onClick={() => setTxType(value)}
                    style={{
                      flex: 1,
                      padding: '6px 0',
                      background: isActive ? activeColor : 'transparent',
                      border: `1px solid ${isActive ? activeColor : 'var(--border-primary)'}`,
                      color: isActive ? '#fff' : 'var(--text-secondary)',
                      borderRadius: '2px',
                      fontFamily: 'var(--font-sans)',
                      fontSize: 12,
                      fontWeight: 600,
                      cursor: 'pointer',
                      transition: 'all 150ms',
                    }}
                  >
                    {label}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Quantity + Price row */}
          <div
            style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12, marginBottom: 20 }}
          >
            <div>
              <label style={labelStyle}>Quantity</label>
              <input
                type="number"
                step="any"
                min="0"
                value={quantity}
                onChange={(e) => setQuantity(e.target.value)}
                placeholder="e.g. 10"
                style={inputStyle}
                required
              />
            </div>
            <div>
              <label style={labelStyle}>Price per unit ({holding.currency})</label>
              <input
                type="number"
                step="any"
                min="0"
                value={price}
                onChange={(e) => setPrice(e.target.value)}
                placeholder="e.g. 150.00"
                style={inputStyle}
                required
              />
            </div>
          </div>

          {/* Date */}
          <div style={{ marginBottom: 20 }}>
            <label style={labelStyle}>Date &amp; Time</label>
            <input
              type="datetime-local"
              value={transactedAt}
              onChange={(e) => setTransactedAt(e.target.value)}
              style={inputStyle}
              required
            />
          </div>

          {/* Error */}
          {error && (
            <div
              style={{
                fontSize: 12,
                color: 'var(--color-loss)',
                fontFamily: 'var(--font-mono)',
                marginBottom: 14,
                padding: '8px 10px',
                background: 'rgba(255,71,87,0.08)',
                border: '1px solid rgba(255,71,87,0.3)',
                borderRadius: '2px',
              }}
            >
              {error}
            </div>
          )}

          {/* Actions */}
          <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end' }}>
            <button
              type="button"
              onClick={handleClose}
              disabled={saving}
              style={{
                padding: '7px 18px',
                background: 'transparent',
                border: '1px solid var(--border-primary)',
                color: 'var(--text-secondary)',
                borderRadius: '2px',
                fontFamily: 'var(--font-sans)',
                fontSize: 12,
                cursor: 'pointer',
              }}
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={saving}
              style={{
                padding: '7px 20px',
                background: 'var(--color-accent)',
                border: 'none',
                color: '#fff',
                borderRadius: '2px',
                fontFamily: 'var(--font-sans)',
                fontSize: 12,
                fontWeight: 600,
                cursor: saving ? 'not-allowed' : 'pointer',
                opacity: saving ? 0.7 : 1,
              }}
            >
              {saving ? 'Saving…' : 'Save'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
