interface SpinnerProps {
  size?: 'sm' | 'md' | 'lg';
}

const SIZE_MAP = { sm: 16, md: 24, lg: 36 };

export function Spinner({ size = 'md' }: SpinnerProps) {
  const px = SIZE_MAP[size];
  return (
    <span
      style={{
        display: 'inline-block',
        width: px,
        height: px,
        borderRadius: '50%',
        border: `2px solid var(--border-primary)`,
        borderTopColor: 'var(--color-accent)',
        animation: 'spin 0.7s linear infinite',
      }}
    />
  );
}
