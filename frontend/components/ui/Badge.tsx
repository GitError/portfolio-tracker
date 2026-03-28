import type { AccountType, AssetType } from '../../types/portfolio';
import { ACCOUNT_TYPE_CONFIG, ASSET_TYPE_CONFIG } from '../../lib/constants';

const BADGE_STYLE = {
  borderRadius: '2px',
  fontSize: '10px',
  fontWeight: 600,
  padding: '1px 6px',
  textTransform: 'uppercase' as const,
  letterSpacing: '0.05em',
  fontFamily: 'var(--font-mono)',
};

interface BadgeProps {
  type: AssetType;
}

export function Badge({ type }: BadgeProps) {
  const config = ASSET_TYPE_CONFIG[type];
  return (
    <span
      style={{
        ...BADGE_STYLE,
        color: config.color,
        border: `1px solid ${config.color}`,
      }}
    >
      {config.label}
    </span>
  );
}

export function AccountBadge({ account }: { account: AccountType }) {
  const config = ACCOUNT_TYPE_CONFIG[account] ?? {
    label: account.toUpperCase(),
    color: 'var(--text-muted)',
  };
  return (
    <span
      style={{
        ...BADGE_STYLE,
        color: config.color,
        border: `1px solid ${config.color}55`,
      }}
    >
      {config.label}
    </span>
  );
}

export function ExchangeBadge({ exchange }: { exchange: string }) {
  if (!exchange) return null;
  return (
    <span
      style={{
        ...BADGE_STYLE,
        color: 'var(--text-secondary)',
        border: '1px solid var(--border-primary)',
      }}
    >
      {exchange}
    </span>
  );
}
