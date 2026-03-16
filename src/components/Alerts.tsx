import { useCallback, useEffect, useState } from 'react';
import { Bell, BellOff, Plus, Trash2, RefreshCw } from 'lucide-react';
import type { AlertDirection, PriceAlert, PriceAlertInput } from '../types/portfolio';
import { formatCurrency } from '../lib/format';
import { EmptyState } from './ui/EmptyState';
import { Select } from './ui/Select';
import { Spinner } from './ui/Spinner';
import { useToast } from './ui/Toast';
import { isTauri, tauriInvoke } from '../lib/tauri';

const MOCK_ALERTS: PriceAlert[] = [
  {
    id: '1',
    symbol: 'AAPL',
    direction: 'above',
    threshold: 200,
    note: 'Take profit',
    triggered: false,
    createdAt: '2026-01-01T00:00:00Z',
  },
  {
    id: '2',
    symbol: 'BTC-USD',
    direction: 'below',
    threshold: 80000,
    note: 'Buy the dip',
    triggered: true,
    createdAt: '2026-01-02T00:00:00Z',
  },
];

interface AddAlertFormProps {
  onAdd: (input: PriceAlertInput) => void;
  onCancel: () => void;
}

function AddAlertForm({ onAdd, onCancel }: AddAlertFormProps) {
  const [symbol, setSymbol] = useState('');
  const [direction, setDirection] = useState<AlertDirection>('above');
  const [threshold, setThreshold] = useState('');
  const [note, setNote] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const t = parseFloat(threshold);
    if (!symbol.trim() || isNaN(t) || t <= 0) return;
    onAdd({
      symbol: symbol.trim().toUpperCase(),
      direction,
      threshold: t,
      note: note.trim(),
    });
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
        New Price Alert
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 10 }}>
        <div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>SYMBOL</div>
          <input
            style={inputStyle}
            value={symbol}
            onChange={(e) => setSymbol(e.target.value)}
            placeholder="e.g. AAPL"
            required
          />
        </div>
        <div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>DIRECTION</div>
          <Select
            value={direction}
            onChange={(value) => setDirection(value as AlertDirection)}
            options={[
              { value: 'above', label: 'Above' },
              { value: 'below', label: 'Below' },
            ]}
          />
        </div>
        <div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>PRICE</div>
          <input
            style={{ ...inputStyle, fontFamily: 'var(--font-mono)' }}
            type="number"
            min="0"
            step="any"
            value={threshold}
            onChange={(e) => setThreshold(e.target.value)}
            placeholder="0.00"
            required
          />
        </div>
      </div>
      <div>
        <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>
          NOTE (OPTIONAL)
        </div>
        <input
          style={inputStyle}
          value={note}
          onChange={(e) => setNote(e.target.value)}
          placeholder="e.g. Take profit target"
        />
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
          Add Alert
        </button>
      </div>
    </form>
  );
}

export function Alerts() {
  const [alerts, setAlerts] = useState<PriceAlert[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const { showToast } = useToast();

  const loadAlerts = useCallback(async () => {
    try {
      if (isTauri()) {
        const data = await tauriInvoke<PriceAlert[]>('get_alerts');
        setAlerts(data);
      } else {
        setAlerts(MOCK_ALERTS);
      }
    } catch (err) {
      showToast(`Failed to load alerts: ${String(err)}`, 'error');
    } finally {
      setLoading(false);
    }
  }, [showToast]);

  useEffect(() => {
    void loadAlerts();
  }, [loadAlerts]);

  const handleAdd = useCallback(
    async (input: PriceAlertInput) => {
      try {
        if (isTauri()) {
          const alert = await tauriInvoke<PriceAlert>('add_alert', { alert: input });
          setAlerts((prev) => [alert, ...prev]);
        } else {
          const mock: PriceAlert = {
            id: Date.now().toString(),
            ...input,
            triggered: false,
            createdAt: new Date().toISOString(),
          };
          setAlerts((prev) => [mock, ...prev]);
        }
        setShowForm(false);
        showToast(`Alert created for ${input.symbol}`, 'success');
      } catch (err) {
        showToast(`Failed to create alert: ${String(err)}`, 'error');
      }
    },
    [showToast]
  );

  const handleDelete = useCallback(
    async (id: string) => {
      try {
        if (isTauri()) {
          await tauriInvoke<boolean>('delete_alert', { id });
        }
        setAlerts((prev) => prev.filter((a) => a.id !== id));
        showToast('Alert deleted', 'success');
      } catch (err) {
        showToast(`Failed to delete alert: ${String(err)}`, 'error');
      }
    },
    [showToast]
  );

  const handleReset = useCallback(
    async (id: string) => {
      try {
        if (isTauri()) {
          await tauriInvoke<boolean>('reset_alert', { id });
        }
        setAlerts((prev) => prev.map((a) => (a.id === id ? { ...a, triggered: false } : a)));
      } catch (err) {
        showToast(`Failed to reset alert: ${String(err)}`, 'error');
      }
    },
    [showToast]
  );

  if (loading) {
    return (
      <div
        style={{
          flex: 1,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <Spinner />
      </div>
    );
  }

  const triggered = alerts.filter((a) => a.triggered);
  const active = alerts.filter((a) => !a.triggered);

  return (
    <div style={{ flex: 1, overflow: 'auto', padding: '24px 32px', maxWidth: 800 }}>
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
            Price Alerts
          </h1>
          <p style={{ fontSize: 13, color: 'var(--text-secondary)', margin: 0 }}>
            Get notified when a price crosses your threshold.
          </p>
        </div>
        <button
          onClick={() => setShowForm(true)}
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
            cursor: 'pointer',
          }}
        >
          <Plus size={14} />
          New Alert
        </button>
      </div>

      {showForm && <AddAlertForm onAdd={handleAdd} onCancel={() => setShowForm(false)} />}

      {/* Triggered alerts */}
      {triggered.length > 0 && (
        <>
          <div
            style={{
              fontSize: 11,
              fontWeight: 600,
              textTransform: 'uppercase',
              letterSpacing: '0.08em',
              color: 'var(--color-warning)',
              marginBottom: 8,
            }}
          >
            Triggered ({triggered.length})
          </div>
          <div
            style={{
              border: '1px solid var(--border-primary)',
              borderRadius: 2,
              marginBottom: 24,
              overflow: 'hidden',
            }}
          >
            {triggered.map((alert, i) => (
              <AlertRow
                key={alert.id}
                alert={alert}
                isLast={i === triggered.length - 1}
                onDelete={handleDelete}
                onReset={handleReset}
              />
            ))}
          </div>
        </>
      )}

      {/* Active alerts */}
      <div
        style={{
          fontSize: 11,
          fontWeight: 600,
          textTransform: 'uppercase',
          letterSpacing: '0.08em',
          color: 'var(--text-muted)',
          marginBottom: 8,
        }}
      >
        Active ({active.length})
      </div>

      {active.length === 0 && !showForm ? (
        <EmptyState message="No active alerts. Click 'New Alert' to create one." />
      ) : (
        <div
          style={{
            border: '1px solid var(--border-primary)',
            borderRadius: 2,
            overflow: 'hidden',
          }}
        >
          {active.map((alert, i) => (
            <AlertRow
              key={alert.id}
              alert={alert}
              isLast={i === active.length - 1}
              onDelete={handleDelete}
              onReset={handleReset}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function AlertRow({
  alert,
  isLast,
  onDelete,
  onReset,
}: {
  alert: PriceAlert;
  isLast: boolean;
  onDelete: (id: string) => void;
  onReset: (id: string) => void;
}) {
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 16,
        padding: '12px 16px',
        background: alert.triggered ? 'rgba(251, 191, 36, 0.05)' : 'var(--bg-surface)',
        borderBottom: isLast ? 'none' : '1px solid var(--border-subtle)',
      }}
    >
      {/* Icon */}
      <div
        style={{
          color: alert.triggered ? 'var(--color-warning)' : 'var(--text-secondary)',
          flexShrink: 0,
        }}
      >
        {alert.triggered ? <Bell size={16} /> : <BellOff size={16} />}
      </div>

      {/* Symbol + direction badge */}
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 2 }}>
          <span
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 13,
              fontWeight: 600,
              color: 'var(--text-primary)',
            }}
          >
            {alert.symbol}
          </span>
          <span
            style={{
              fontSize: 10,
              fontWeight: 600,
              textTransform: 'uppercase',
              letterSpacing: '0.06em',
              padding: '1px 6px',
              borderRadius: 2,
              background:
                alert.direction === 'above' ? 'rgba(0, 212, 170, 0.15)' : 'rgba(255, 71, 87, 0.15)',
              color: alert.direction === 'above' ? 'var(--color-gain)' : 'var(--color-loss)',
            }}
          >
            {alert.direction} {formatCurrency(alert.threshold, 'USD')}
          </span>
          {alert.triggered && (
            <span
              style={{
                fontSize: 10,
                fontWeight: 600,
                textTransform: 'uppercase',
                letterSpacing: '0.06em',
                padding: '1px 6px',
                borderRadius: 2,
                background: 'rgba(251, 191, 36, 0.15)',
                color: 'var(--color-warning)',
              }}
            >
              TRIGGERED
            </span>
          )}
        </div>
        {alert.note && (
          <div style={{ fontSize: 12, color: 'var(--text-secondary)' }}>{alert.note}</div>
        )}
      </div>

      {/* Actions */}
      <div style={{ display: 'flex', gap: 4, flexShrink: 0 }}>
        {alert.triggered && (
          <button
            onClick={() => onReset(alert.id)}
            title="Reset alert"
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
          >
            <RefreshCw size={12} />
          </button>
        )}
        <button
          onClick={() => onDelete(alert.id)}
          title="Delete alert"
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
            (e.currentTarget as HTMLButtonElement).style.borderColor = 'var(--border-primary)';
          }}
        >
          <Trash2 size={12} />
        </button>
      </div>
    </div>
  );
}
