import i18next from './i18n';

const INVALID_NUMBER = '—';

function isValidNumber(value: number | null | undefined): value is number {
  return value != null && Number.isFinite(value) && !Number.isNaN(value);
}

export function formatCurrency(
  amount: number | null | undefined,
  currency = 'CAD',
  localeOverride?: string
): string {
  if (!isValidNumber(amount)) return INVALID_NUMBER;
  const locale = localeOverride || i18next.language || 'en';
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

export function formatPercent(decimal: number | null | undefined, localeOverride?: string): string {
  if (!isValidNumber(decimal)) return INVALID_NUMBER;
  const locale = localeOverride || i18next.language || 'en';
  return new Intl.NumberFormat(locale, {
    style: 'percent',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
    signDisplay: 'always',
  }).format(decimal / 100);
}

/** Formats a nullable target weight percentage. Null/undefined means "no target set" (→ "—"); 0 is an explicit target and renders as "0.0%". */
export function formatTargetWeight(
  weight: number | null | undefined,
  localeOverride?: string
): string {
  if (!isValidNumber(weight)) return INVALID_NUMBER;
  const locale = localeOverride || i18next.language || 'en';
  return new Intl.NumberFormat(locale, {
    style: 'percent',
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(weight / 100);
}

export function formatNumber(
  n: number | null | undefined,
  decimals = 2,
  localeOverride?: string
): string {
  if (!isValidNumber(n)) return INVALID_NUMBER;
  const locale = localeOverride || i18next.language || 'en';
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

/** Formats a date string as "Dec 2025". Returns "—" for null/invalid. */
export function formatMonthYear(
  dateStr: string | null | undefined,
  localeOverride?: string
): string {
  if (!dateStr) return '—';
  const d = new Date(dateStr);
  if (isNaN(d.getTime())) return '—';
  const locale = localeOverride || i18next.language || 'en';
  return d.toLocaleDateString(locale, { month: 'short', year: 'numeric' });
}

/** Formats a date string as "Jan 5, 2025". Returns "—" for null/invalid. */
export function formatShortDate(
  dateStr: string | null | undefined,
  localeOverride?: string
): string {
  if (!dateStr) return '—';
  const d = new Date(dateStr);
  if (isNaN(d.getTime())) return '—';
  const locale = localeOverride || i18next.language || 'en';
  return d.toLocaleDateString(locale, { month: 'short', day: 'numeric', year: 'numeric' });
}

export function formatCompact(
  n: number | null | undefined,
  currency = 'CAD',
  localeOverride?: string
): string {
  if (!isValidNumber(n)) return INVALID_NUMBER;
  const locale = localeOverride || i18next.language || 'en';
  const compact = Math.abs(n) >= 1_000;
  return new Intl.NumberFormat(locale, {
    style: 'currency',
    currency,
    notation: compact ? 'compact' : 'standard',
    minimumFractionDigits: compact ? undefined : 2,
    maximumFractionDigits: compact ? 1 : 2,
  }).format(n);
}
