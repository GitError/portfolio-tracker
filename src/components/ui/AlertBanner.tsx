import { useEffect, useState } from 'react';
import { Bell, X } from 'lucide-react';
import { formatNumber } from '../../lib/format';

interface AlertTriggeredPayload {
  alertId: number;
  holdingId: string;
  symbol: string;
  alertType: 'above' | 'below';
  targetPrice: number;
  currentPrice: number;
  currency: string;
}

interface AlertEntry {
  id: string;
  payload: AlertTriggeredPayload;
}

export function AlertBanner() {
  const [alerts, setAlerts] = useState<AlertEntry[]>([]);

  useEffect(() => {
    // No-op in browser mode when Tauri is not available
    if (
      typeof window === 'undefined' ||
      !(window as unknown as Record<string, unknown>).__TAURI__
    ) {
      return;
    }

    let unlisten: (() => void) | undefined;

    import('@tauri-apps/api/event')
      .then(({ listen }) => {
        return listen<AlertTriggeredPayload>('price-alert-triggered', (event) => {
          const entry: AlertEntry = {
            id: `${event.payload.alertId}-${Date.now()}`,
            payload: event.payload,
          };
          setAlerts((prev) => [...prev, entry]);

          // Auto-dismiss after 5 seconds
          setTimeout(() => {
            setAlerts((prev) => prev.filter((a) => a.id !== entry.id));
          }, 5000);
        });
      })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((e) => {
        console.warn('AlertBanner: failed to register event listener:', e);
      });

    return () => {
      unlisten?.();
    };
  }, []);

  function dismiss(id: string) {
    setAlerts((prev) => prev.filter((a) => a.id !== id));
  }

  if (alerts.length === 0) return null;

  return (
    <div
      style={{
        position: 'fixed',
        bottom: 24,
        right: 24,
        zIndex: 2000,
        display: 'flex',
        flexDirection: 'column',
        gap: 8,
        maxWidth: 380,
      }}
    >
      {alerts.map((entry) => {
        const { payload } = entry;
        const priceStr = `${formatNumber(payload.targetPrice, 2)} ${payload.currency}`;
        const message = `${payload.symbol} crossed ${priceStr} (${payload.alertType} threshold)`;

        return (
          <div
            key={entry.id}
            role="alert"
            style={{
              display: 'flex',
              alignItems: 'flex-start',
              gap: 10,
              padding: '12px 14px',
              background: 'var(--bg-surface)',
              border: '1px solid var(--color-warning)',
              borderLeft: '4px solid var(--color-warning)',
              borderRadius: '2px',
              boxShadow: '0 4px 16px rgba(0,0,0,0.4)',
              animation: 'fadeIn 200ms ease',
            }}
          >
            <Bell
              size={14}
              style={{ color: 'var(--color-warning)', flexShrink: 0, marginTop: 1 }}
            />
            <span
              style={{
                flex: 1,
                fontFamily: 'var(--font-mono)',
                fontSize: 12,
                color: 'var(--text-primary)',
                lineHeight: 1.5,
              }}
            >
              {message}
            </span>
            <button
              onClick={() => dismiss(entry.id)}
              aria-label="Dismiss alert"
              style={{
                background: 'none',
                border: 'none',
                color: 'var(--text-muted)',
                cursor: 'pointer',
                padding: 0,
                display: 'flex',
                alignItems: 'center',
                flexShrink: 0,
              }}
            >
              <X size={13} />
            </button>
          </div>
        );
      })}
    </div>
  );
}
