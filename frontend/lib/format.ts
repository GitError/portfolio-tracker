import i18next from './i18n';

const INVALID_NUMBER = '—';

function isValidNumber(value: number | null | undefined): value is number {
  return value != null && Number.isFinite(value) && !Number.isNaN(value);
}

export function formatCurrency(amount: number | null | undefined, currency = 'CAD'): string {
  if (!isValidNumber(amount)) return INVALID_NUMBER;
  const locale = i18next.language || 'en';
  return (
    new Intl.NumberFormat(locale, {
      style: 'decimal',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(amount) +
    ' ' +
    currency
  );
}

export function formatPercent(decimal: number | null | undefined): string {
  if (!isValidNumber(decimal)) return INVALID_NUMBER;
  const sign = decimal >= 0 ? '+' : '';
  return `${sign}${decimal.toFixed(2)}%`;
}

export function formatNumber(n: number | null | undefined, decimals = 2): string {
  if (!isValidNumber(n)) return INVALID_NUMBER;
  const locale = i18next.language || 'en';
  return new Intl.NumberFormat(locale, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(n);
}

/** Returns true if the price timestamp is older than the given threshold (default 2 hours). */
export function isPriceStale(
  updatedAt: string | null | undefined,
  thresholdMs = 2 * 60 * 60 * 1000
): boolean {
  if (!updatedAt) return true;
  return Date.now() - new Date(updatedAt).getTime() > thresholdMs;
}

export function formatCompact(n: number | null | undefined): string {
  if (!isValidNumber(n)) return INVALID_NUMBER;
  const abs = Math.abs(n);
  const sign = n < 0 ? '-' : '';
  if (abs >= 1_000_000) return `${sign}$${(abs / 1_000_000).toFixed(1)}M`;
  if (abs >= 1_000) return `${sign}$${(abs / 1_000).toFixed(1)}K`;
  return `${sign}$${abs.toFixed(2)}`;
}
