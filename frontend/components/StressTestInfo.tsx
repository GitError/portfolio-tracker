import { useEffect } from 'react';
import { X } from 'lucide-react';
import { formatPercent } from '../lib/format';
import type { StressScenarioInfo } from '../types/portfolio';

interface Props {
  scenario: StressScenarioInfo | null;
  isOpen: boolean;
  onClose: () => void;
}

const OVERLAY: React.CSSProperties = {
  position: 'fixed',
  inset: 0,
  background: 'rgba(0,0,0,0.6)',
  backdropFilter: 'blur(4px)',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  zIndex: 1000,
  padding: 20,
};

const PANEL: React.CSSProperties = {
  width: '100%',
  maxWidth: 560,
  background: 'var(--bg-surface)',
  border: '1px solid var(--border-primary)',
  padding: 24,
};

export function StressTestInfo({ scenario, isOpen, onClose }: Props) {
  useEffect(() => {
    if (!isOpen) return;
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') onClose();
    }
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen || !scenario) return null;

  return (
    <div style={OVERLAY} onClick={(event) => event.target === event.currentTarget && onClose()}>
      <div style={PANEL}>
        <div
          style={{
            display: 'flex',
            alignItems: 'flex-start',
            justifyContent: 'space-between',
            gap: 16,
            marginBottom: 18,
          }}
        >
          <div>
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
              Scenario Info
            </div>
            <h2
              style={{
                margin: 0,
                fontFamily: 'var(--font-sans)',
                fontSize: 18,
                color: 'var(--text-primary)',
              }}
            >
              {scenario.name}
            </h2>
          </div>
          <button
            onClick={onClose}
            style={{
              background: 'transparent',
              border: '1px solid var(--border-primary)',
              color: 'var(--text-secondary)',
              width: 32,
              height: 32,
              display: 'grid',
              placeItems: 'center',
              cursor: 'pointer',
            }}
          >
            <X size={14} />
          </button>
        </div>

        <p
          style={{
            margin: 0,
            color: 'var(--text-secondary)',
            fontSize: 14,
            lineHeight: 1.6,
          }}
        >
          {scenario.description}
        </p>

        <div style={{ marginTop: 18 }}>
          <div
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 10,
              color: 'var(--text-muted)',
              textTransform: 'uppercase',
              letterSpacing: '0.08em',
              marginBottom: 8,
            }}
          >
            Shock Breakdown
          </div>
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
              gap: 8,
            }}
          >
            {Object.entries(scenario.shocks).map(([key, value]) => (
              <div
                key={key}
                style={{
                  border: '1px solid var(--border-subtle)',
                  padding: '10px 12px',
                  background: 'var(--bg-surface-alt)',
                }}
              >
                <div
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: 10,
                    color: 'var(--text-muted)',
                    textTransform: 'uppercase',
                    letterSpacing: '0.06em',
                    marginBottom: 4,
                  }}
                >
                  {key.replace(/_/g, '/')}
                </div>
                <div
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: 14,
                    color: value >= 0 ? 'var(--color-gain)' : 'var(--color-loss)',
                    fontWeight: 700,
                  }}
                >
                  {formatPercent(value * 100)}
                </div>
              </div>
            ))}
          </div>
        </div>

        <div style={{ marginTop: 18 }}>
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
            Historical Parallel
          </div>
          <div
            style={{
              color: 'var(--text-primary)',
              fontSize: 13,
              fontFamily: 'var(--font-mono)',
            }}
          >
            {scenario.historicalParallel}
          </div>
        </div>
      </div>
    </div>
  );
}
