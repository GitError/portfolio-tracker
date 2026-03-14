import type { AssetType } from '../types/portfolio';

export function pnlColor(value: number): string {
  if (value > 0) return 'var(--color-gain)';
  if (value < 0) return 'var(--color-loss)';
  return 'var(--text-secondary)';
}

export function pnlClass(value: number): string {
  if (value > 0) return 'text-gain';
  if (value < 0) return 'text-loss';
  return 'text-secondary';
}

export function assetTypeColor(type: AssetType): string {
  switch (type) {
    case 'stock':
      return 'var(--color-stock)';
    case 'etf':
      return 'var(--color-etf)';
    case 'crypto':
      return 'var(--color-crypto)';
    case 'cash':
      return 'var(--color-cash)';
  }
}
