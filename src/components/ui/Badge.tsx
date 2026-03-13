import type { AssetType } from '../../types/portfolio';
import { ASSET_TYPE_CONFIG } from '../../lib/constants';

interface BadgeProps {
  type: AssetType;
}

export function Badge({ type }: BadgeProps) {
  const config = ASSET_TYPE_CONFIG[type];
  return (
    <span
      style={{
        color: config.color,
        border: `1px solid ${config.color}`,
        borderRadius: '2px',
        fontSize: '10px',
        fontWeight: 600,
        padding: '1px 6px',
        textTransform: 'uppercase',
        letterSpacing: '0.05em',
        fontFamily: 'var(--font-mono)',
      }}
    >
      {config.label}
    </span>
  );
}
