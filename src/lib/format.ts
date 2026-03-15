const INVALID_NUMBER = '—';

function isInvalidNumber(value: number): boolean {
  return value == null || !isFinite(value) || isNaN(value);
}

export function formatCurrency(amount: number, currency = 'CAD'): string {
  if (isInvalidNumber(amount)) return INVALID_NUMBER;
  return (
    new Intl.NumberFormat('en-CA', {
      style: 'decimal',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(amount) +
    ' ' +
    currency
  );
}

export function formatPercent(decimal: number): string {
  if (isInvalidNumber(decimal)) return INVALID_NUMBER;
  const sign = decimal >= 0 ? '+' : '';
  return `${sign}${decimal.toFixed(2)}%`;
}

export function formatNumber(n: number, decimals = 2): string {
  if (isInvalidNumber(n)) return INVALID_NUMBER;
  return new Intl.NumberFormat('en-CA', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(n);
}

export function formatCompact(n: number): string {
  if (isInvalidNumber(n)) return INVALID_NUMBER;
  const abs = Math.abs(n);
  const sign = n < 0 ? '-' : '';
  if (abs >= 1_000_000) return `${sign}$${(abs / 1_000_000).toFixed(1)}M`;
  if (abs >= 1_000) return `${sign}$${(abs / 1_000).toFixed(1)}K`;
  return `${sign}$${abs.toFixed(2)}`;
}
