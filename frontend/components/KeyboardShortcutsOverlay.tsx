import { useEffect } from 'react';
import { X } from 'lucide-react';

interface ShortcutRow {
  keys: string[];
  action: string;
}

const SHORTCUTS: ShortcutRow[] = [
  { keys: ['⌘', 'R'], action: 'Refresh prices' },
  { keys: ['⌘', 'N'], action: 'Add holding' },
  { keys: ['⌘', '1'], action: 'Go to Dashboard' },
  { keys: ['⌘', '2'], action: 'Go to Holdings' },
  { keys: ['⌘', '3'], action: 'Go to Performance' },
  { keys: ['⌘', '4'], action: 'Go to Stress Test' },
  { keys: ['?'], action: 'Toggle this help panel' },
  { keys: ['Esc'], action: 'Close overlays / modals' },
];

interface KeyboardShortcutsOverlayProps {
  isOpen: boolean;
  onClose: () => void;
}

export function KeyboardShortcutsOverlay({ isOpen, onClose }: KeyboardShortcutsOverlayProps) {
  useEffect(() => {
    if (!isOpen) return;

    function handleKeyDown(e: KeyboardEvent): void {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      onClick={onClose}
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 100,
        background: 'rgba(0, 0, 0, 0.6)',
        backdropFilter: 'blur(4px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
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
          position: 'relative',
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
          <h2
            style={{
              fontFamily: 'var(--font-sans)',
              fontWeight: 600,
              fontSize: 15,
              color: 'var(--text-primary)',
              letterSpacing: '-0.01em',
              margin: 0,
            }}
          >
            Keyboard Shortcuts
          </h2>
          <button
            onClick={onClose}
            aria-label="Close keyboard shortcuts"
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              background: 'transparent',
              border: '1px solid var(--border-primary)',
              color: 'var(--text-secondary)',
              borderRadius: '2px',
              cursor: 'pointer',
              padding: 4,
            }}
          >
            <X size={14} />
          </button>
        </div>

        {/* Shortcuts table */}
        <table
          style={{
            width: '100%',
            borderCollapse: 'collapse',
            fontFamily: 'var(--font-sans)',
          }}
        >
          <thead>
            <tr>
              <th
                style={{
                  textAlign: 'left',
                  fontSize: 10,
                  fontFamily: 'var(--font-mono)',
                  color: 'var(--text-secondary)',
                  textTransform: 'uppercase',
                  letterSpacing: '0.08em',
                  fontWeight: 600,
                  padding: '4px 0 8px 0',
                  borderBottom: '1px solid var(--border-primary)',
                  width: '40%',
                }}
              >
                Keys
              </th>
              <th
                style={{
                  textAlign: 'left',
                  fontSize: 10,
                  fontFamily: 'var(--font-mono)',
                  color: 'var(--text-secondary)',
                  textTransform: 'uppercase',
                  letterSpacing: '0.08em',
                  fontWeight: 600,
                  padding: '4px 0 8px 12px',
                  borderBottom: '1px solid var(--border-primary)',
                }}
              >
                Action
              </th>
            </tr>
          </thead>
          <tbody>
            {SHORTCUTS.map((row, index) => (
              <tr
                key={index}
                style={{
                  background: index % 2 === 0 ? 'transparent' : 'var(--bg-surface-alt)',
                }}
              >
                <td
                  style={{
                    padding: '8px 0',
                    borderBottom: '1px solid var(--border-subtle)',
                    verticalAlign: 'middle',
                  }}
                >
                  <span style={{ display: 'flex', alignItems: 'center', gap: 4, flexWrap: 'wrap' }}>
                    {row.keys.map((key, ki) => (
                      <kbd
                        key={ki}
                        style={{
                          display: 'inline-block',
                          fontFamily: 'var(--font-mono)',
                          fontSize: 11,
                          fontWeight: 500,
                          color: 'var(--text-primary)',
                          background: 'var(--bg-surface-hover)',
                          border: '1px solid var(--border-primary)',
                          borderRadius: '2px',
                          padding: '2px 6px',
                          lineHeight: '1.4',
                        }}
                      >
                        {key}
                      </kbd>
                    ))}
                  </span>
                </td>
                <td
                  style={{
                    padding: '8px 0 8px 12px',
                    fontSize: 13,
                    color: 'var(--text-secondary)',
                    borderBottom: '1px solid var(--border-subtle)',
                    verticalAlign: 'middle',
                  }}
                >
                  {row.action}
                </td>
              </tr>
            ))}
          </tbody>
        </table>

        {/* Footer hint */}
        <p
          style={{
            marginTop: 16,
            fontSize: 11,
            color: 'var(--text-muted)',
            fontFamily: 'var(--font-mono)',
            textAlign: 'center',
          }}
        >
          Shortcuts are disabled when focus is inside an input field.
        </p>
      </div>
    </div>
  );
}
