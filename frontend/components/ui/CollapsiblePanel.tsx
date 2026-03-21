import { useState } from 'react';
import { ChevronDown } from 'lucide-react';

export interface CollapsiblePanelProps {
  title: string;
  defaultExpanded?: boolean;
  children: React.ReactNode;
}

export function CollapsiblePanel({
  title,
  defaultExpanded = true,
  children,
}: CollapsiblePanelProps) {
  const [expanded, setExpanded] = useState(defaultExpanded);

  return (
    <div
      style={{
        background: 'var(--bg-surface)',
        border: '1px solid var(--border-primary)',
        marginBottom: 1,
      }}
    >
      {/* Header */}
      <button
        type="button"
        onClick={() => setExpanded((prev) => !prev)}
        style={{
          width: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '14px 20px',
          background: 'transparent',
          border: 'none',
          borderBottom: expanded ? '1px solid var(--border-subtle)' : 'none',
          cursor: 'pointer',
          color: 'var(--text-secondary)',
          transition: 'color 150ms',
        }}
        onMouseEnter={(e) => {
          (e.currentTarget as HTMLButtonElement).style.color = 'var(--text-primary)';
        }}
        onMouseLeave={(e) => {
          (e.currentTarget as HTMLButtonElement).style.color = 'var(--text-secondary)';
        }}
      >
        <span
          style={{
            fontFamily: 'var(--font-mono)',
            fontSize: 10,
            fontWeight: 600,
            textTransform: 'uppercase',
            letterSpacing: '0.1em',
            color: 'var(--text-muted)',
          }}
        >
          {title}
        </span>
        <ChevronDown
          size={14}
          style={{
            flexShrink: 0,
            transform: `rotate(${expanded ? 0 : -90}deg)`,
            transition: 'transform 200ms ease',
            color: 'var(--text-muted)',
          }}
        />
      </button>

      {/* Content */}
      {expanded && <div style={{ padding: '16px 20px' }}>{children}</div>}
    </div>
  );
}
