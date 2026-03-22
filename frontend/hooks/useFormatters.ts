import { useTranslation } from 'react-i18next';
import { isPriceStale } from '../lib/format';

const INVALID_NUMBER = '—';

function isValidNumber(value: number | null | undefined): value is number {
  return value != null && Number.isFinite(value) && !Number.isNaN(value);
}

export interface Formatters {
  formatCurrency: (amount: number | null | undefined, currency?: string) => string;
  formatNumber: (n: number | null | undefined, decimals?: number) => string;
  formatPercent: (decimal: number | null | undefined) => string;
  formatCompact: (n: number | null | undefined) => string;
}

/**
 * Returns formatter functions bound to the current i18n language.
 *
 * Unlike the standalone functions in `lib/format.ts` (which read `i18next.language`
 * imperatively), this hook subscribes to language changes via `useTranslation()`.
 * Memoised components that destructure from `useFormatters()` will therefore
 * re-render — and reformat values — whenever the user switches language.
 */
export function useFormatters(): Formatters {
  // `useTranslation` returns a new object reference when the language changes,
  // which causes any component calling this hook to re-render automatically.
  const { i18n } = useTranslation();
  const locale = i18n.language || 'en';

  const formatCurrency = (amount: number | null | undefined, currency = 'CAD'): string => {
    if (!isValidNumber(amount)) return INVALID_NUMBER;
    return (
      new Intl.NumberFormat(locale, {
        style: 'decimal',
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      }).format(amount) +
      ' ' +
      currency
    );
  };

  const formatNumber = (n: number | null | undefined, decimals = 2): string => {
    if (!isValidNumber(n)) return INVALID_NUMBER;
    return new Intl.NumberFormat(locale, {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    }).format(n);
  };

  const formatPercent = (decimal: number | null | undefined): string => {
    if (!isValidNumber(decimal)) return INVALID_NUMBER;
    const sign = decimal >= 0 ? '+' : '';
    return `${sign}${decimal.toFixed(2)}%`;
  };

  const formatCompact = (n: number | null | undefined): string => {
    if (!isValidNumber(n)) return INVALID_NUMBER;
    const abs = Math.abs(n);
    const sign = n < 0 ? '-' : '';
    if (abs >= 1_000_000) return `${sign}$${(abs / 1_000_000).toFixed(1)}M`;
    if (abs >= 1_000) return `${sign}$${(abs / 1_000).toFixed(1)}K`;
    return `${sign}$${abs.toFixed(2)}`;
  };

  return { formatCurrency, formatNumber, formatPercent, formatCompact };
}

// Re-export isPriceStale for consumers that import formatting helpers from hooks.
export { isPriceStale };
