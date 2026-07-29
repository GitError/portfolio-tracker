import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { formatCurrency, formatNumber } from '../lib/format';

/** Reactive wrapper around `formatCurrency` — re-renders callers when the active language changes. */
export function useFormatCurrency() {
  const { i18n } = useTranslation();
  return useCallback(
    (amount: number | null | undefined, currency?: string) =>
      formatCurrency(amount, currency, i18n.language),
    [i18n.language]
  );
}

/** Reactive wrapper around `formatNumber` — re-renders callers when the active language changes. */
export function useFormatNumber() {
  const { i18n } = useTranslation();
  return useCallback(
    (n: number | null | undefined, decimals?: number) => formatNumber(n, decimals, i18n.language),
    [i18n.language]
  );
}
