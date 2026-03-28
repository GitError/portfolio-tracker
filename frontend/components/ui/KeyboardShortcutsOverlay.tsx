import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';

interface ShortcutRow {
  keys: string[];
  descriptionKey: string;
}

const SHORTCUTS: ShortcutRow[] = [
  { keys: ['⌘R', 'Ctrl+R'], descriptionKey: 'keyboard.refreshPrices' },
  { keys: ['⌘N', 'Ctrl+N'], descriptionKey: 'keyboard.addHolding' },
  { keys: ['⌘E', 'Ctrl+E'], descriptionKey: 'keyboard.exportCsv' },
  { keys: ['⌘1', 'Ctrl+1'], descriptionKey: 'keyboard.goDashboard' },
  { keys: ['⌘2', 'Ctrl+2'], descriptionKey: 'keyboard.goHoldings' },
  { keys: ['⌘3', 'Ctrl+3'], descriptionKey: 'keyboard.goPerformance' },
  { keys: ['⌘4', 'Ctrl+4'], descriptionKey: 'keyboard.goStressTest' },
  { keys: ['⌘,', 'Ctrl+,'], descriptionKey: 'keyboard.goSettings' },
  { keys: ['?'], descriptionKey: 'keyboard.toggleHelp' },
  { keys: ['Esc'], descriptionKey: 'keyboard.closeOverlay' },
];

interface KeyboardShortcutsOverlayProps {
  isOpen: boolean;
  onClose: () => void;
}

export function KeyboardShortcutsOverlay({ isOpen, onClose }: KeyboardShortcutsOverlayProps) {
  const { t } = useTranslation();

  useEffect(() => {
    if (!isOpen) return;

    function handleKey(e: KeyboardEvent) {
      if (e.key === 'Escape' || e.key === '?') {
        e.preventDefault();
        onClose();
      }
    }

    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={t('keyboard.title')}
      onClick={onClose}
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 100,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0, 0, 0, 0.6)',
        backdropFilter: 'blur(4px)',
      }}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-primary)',
          borderRadius: '2px',
          width: '100%',
          maxWidth: 480,
          padding: '24px',
        }}
      >
        {/* Header */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            marginBottom: 20,
          }}
        >
          <span
            style={{
              fontFamily: 'var(--font-sans)',
              fontWeight: 600,
              fontSize: 14,
              color: 'var(--text-primary)',
              letterSpacing: '-0.01em',
            }}
          >
            {t('keyboard.title')}
          </span>
          <button
            onClick={onClose}
            aria-label={t('keyboard.closeOverlay')}
            style={{
              background: 'none',
              border: '1px solid var(--border-primary)',
              color: 'var(--text-muted)',
              cursor: 'pointer',
              padding: '2px 8px',
              borderRadius: '2px',
              fontFamily: 'var(--font-mono)',
              fontSize: 11,
            }}
          >
            Esc
          </button>
        </div>

        {/* Shortcut rows */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
          {SHORTCUTS.map((shortcut) => (
            <div
              key={shortcut.descriptionKey}
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                padding: '8px 10px',
                background: 'var(--bg-surface-alt)',
                borderRadius: '2px',
              }}
            >
              <span
                style={{
                  fontFamily: 'var(--font-sans)',
                  fontSize: 12,
                  color: 'var(--text-secondary)',
                }}
              >
                {t(shortcut.descriptionKey)}
              </span>
              <div style={{ display: 'flex', gap: 4, alignItems: 'center' }}>
                {shortcut.keys.map((key, index) => (
                  <span key={key} style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                    {index > 0 && (
                      <span
                        style={{
                          fontFamily: 'var(--font-mono)',
                          fontSize: 10,
                          color: 'var(--text-muted)',
                        }}
                      >
                        /
                      </span>
                    )}
                    <kbd
                      style={{
                        fontFamily: 'var(--font-mono)',
                        fontSize: 11,
                        fontWeight: 500,
                        color: 'var(--text-primary)',
                        background: 'var(--bg-primary)',
                        border: '1px solid var(--border-primary)',
                        borderRadius: '2px',
                        padding: '2px 7px',
                        letterSpacing: '0.02em',
                        lineHeight: 1.6,
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {key}
                    </kbd>
                  </span>
                ))}
              </div>
            </div>
          ))}
        </div>

        {/* Footer hint */}
        <div
          style={{
            marginTop: 16,
            paddingTop: 12,
            borderTop: '1px solid var(--border-subtle)',
            fontFamily: 'var(--font-mono)',
            fontSize: 10,
            color: 'var(--text-muted)',
            textAlign: 'center',
            letterSpacing: '0.04em',
          }}
        >
          {t('keyboard.disabledInInputs')}
        </div>
      </div>
    </div>
  );
}
