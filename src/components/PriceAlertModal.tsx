import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Bell } from 'lucide-react';
import type { CreateAlertRequest, Holding, HoldingWithPrice, PriceAlert } from '../types/portfolio';
import { formatNumber } from '../lib/format';

interface PriceAlertModalProps {
  holding: Holding | HoldingWithPrice;
  isOpen: boolean;
  onClose: () => void;
  onSaved: () => void;
}

export function PriceAlertModal({ holding, isOpen, onClose, onSaved }: PriceAlertModalProps) {
  const [alertType, setAlertType] = useState<'above' | 'below'>('above');
  const [targetPrice, setTargetPrice] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const currentPrice = 'currentPrice' in holding ? holding.currentPrice : undefined;

  function handleOverlayClick(e: React.MouseEvent<HTMLDivElement>) {
    if (e.target === e.currentTarget) {
      handleClose();
    }
  }

  function handleClose() {
    setTargetPrice('');
    setAlertType('above');
    setError(null);
    onClose();
  }

  async function handleSave() {
    const parsed = parseFloat(targetPrice);
    if (!targetPrice || isNaN(parsed) || parsed <= 0) {
      setError('Target price must be a positive number.');
      return;
    }

    setSaving(true);
    setError(null);

    try {
      const request: CreateAlertRequest = {
        holdingId: holding.id,
        alertType,
        targetPrice: parsed,
        currency: holding.currency,
      };

      await invoke<PriceAlert>('add_price_alert', { alert: request });
      onSaved();
      handleClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div
      onClick={handleOverlayClick}
      style={{
        position: 'fixed',
        inset: 0,
        background: 'rgba(0,0,0,0.6)',
        backdropFilter: 'blur(4px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
      }}
    >
      <div
        style={{
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-primary)',
          borderRadius: '2px',
          width: '100%',
          maxWidth: 420,
          padding: '24px',
        }}
      >
        {/* Header */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 10,
            marginBottom: 20,
          }}
        >
          <Bell size={16} style={{ color: 'var(--color-warning)' }} />
          <span
            style={{
              fontFamily: 'var(--font-sans)',
              fontWeight: 600,
              fontSize: 14,
              color: 'var(--text-primary)',
            }}
          >
            Set Price Alert
          </span>
          <span
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 12,
              color: 'var(--text-secondary)',
              marginLeft: 4,
            }}
          >
            {holding.symbol}
          </span>
        </div>

        {currentPrice !== undefined && (
          <div
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 11,
              color: 'var(--text-secondary)',
              marginBottom: 16,
            }}
          >
            Current price:{' '}
            <span style={{ color: 'var(--text-primary)' }}>
              {formatNumber(currentPrice, 2)} {holding.currency}
            </span>
          </div>
        )}

        {/* Alert type toggle */}
        <div style={{ marginBottom: 16 }}>
          <div
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 10,
              color: 'var(--text-secondary)',
              textTransform: 'uppercase',
              letterSpacing: '0.08em',
              marginBottom: 8,
            }}
          >
            Alert Type
          </div>
          <div style={{ display: 'flex', gap: 0 }}>
            {(['above', 'below'] as const).map((type) => (
              <button
                key={type}
                onClick={() => setAlertType(type)}
                style={{
                  flex: 1,
                  padding: '8px 0',
                  background: alertType === type ? 'var(--color-accent)' : 'var(--bg-surface)',
                  border: '1px solid var(--border-primary)',
                  borderRight: type === 'above' ? 'none' : '1px solid var(--border-primary)',
                  color: alertType === type ? '#fff' : 'var(--text-secondary)',
                  fontFamily: 'var(--font-mono)',
                  fontSize: 12,
                  fontWeight: alertType === type ? 600 : 400,
                  cursor: 'pointer',
                  textTransform: 'capitalize',
                  transition: 'background 150ms, color 150ms',
                }}
              >
                {type}
              </button>
            ))}
          </div>
        </div>

        {/* Target price input */}
        <div style={{ marginBottom: 20 }}>
          <label
            style={{
              display: 'block',
              fontFamily: 'var(--font-mono)',
              fontSize: 10,
              color: 'var(--text-secondary)',
              textTransform: 'uppercase',
              letterSpacing: '0.08em',
              marginBottom: 8,
            }}
          >
            Target Price ({holding.currency})
          </label>
          <input
            type="number"
            min="0.000001"
            step="any"
            value={targetPrice}
            onChange={(e) => {
              setTargetPrice(e.target.value);
              setError(null);
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void handleSave();
              if (e.key === 'Escape') handleClose();
            }}
            placeholder="e.g. 195.00"
            autoFocus
            style={{
              width: '100%',
              background: 'var(--bg-surface-alt)',
              border: error ? '1px solid var(--color-loss)' : '1px solid var(--border-primary)',
              color: 'var(--text-primary)',
              padding: '8px 10px',
              fontSize: 13,
              fontFamily: 'var(--font-mono)',
              borderRadius: '2px',
              outline: 'none',
              boxSizing: 'border-box',
            }}
          />
        </div>

        {/* Error */}
        {error && (
          <div
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 11,
              color: 'var(--color-loss)',
              marginBottom: 16,
            }}
          >
            {error}
          </div>
        )}

        {/* Actions */}
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <button
            onClick={handleClose}
            disabled={saving}
            style={{
              padding: '7px 16px',
              background: 'transparent',
              border: '1px solid var(--border-primary)',
              color: 'var(--text-secondary)',
              borderRadius: '2px',
              fontFamily: 'var(--font-mono)',
              fontSize: 12,
              cursor: saving ? 'not-allowed' : 'pointer',
            }}
          >
            Cancel
          </button>
          <button
            onClick={() => void handleSave()}
            disabled={saving}
            style={{
              padding: '7px 16px',
              background: saving ? 'var(--text-muted)' : 'var(--color-accent)',
              border: 'none',
              color: '#fff',
              borderRadius: '2px',
              fontFamily: 'var(--font-mono)',
              fontSize: 12,
              fontWeight: 600,
              cursor: saving ? 'not-allowed' : 'pointer',
            }}
          >
            {saving ? 'Saving...' : 'Set Alert'}
          </button>
        </div>
      </div>
    </div>
  );
}
