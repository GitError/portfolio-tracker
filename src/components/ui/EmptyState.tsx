interface EmptyStateProps {
  message: string;
  action?: { label: string; onClick: () => void };
}

export function EmptyState({ message, action }: EmptyStateProps) {
  return (
    <div style={{ textAlign: 'center', padding: '48px 24px' }}>
      <p
        style={{
          fontFamily: 'var(--font-mono)',
          color: 'var(--text-secondary)',
          fontSize: '13px',
          letterSpacing: '0.05em',
        }}
      >
        {'> '}{message.toUpperCase()}
      </p>
      {action && (
        <button
          onClick={action.onClick}
          style={{
            marginTop: '16px',
            padding: '6px 16px',
            background: 'transparent',
            border: '1px solid var(--color-accent)',
            color: 'var(--color-accent)',
            borderRadius: '2px',
            fontFamily: 'var(--font-mono)',
            fontSize: '12px',
            cursor: 'pointer',
          }}
        >
          {action.label}
        </button>
      )}
    </div>
  );
}
