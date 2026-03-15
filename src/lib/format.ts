export function formatCurrency(amount: number | null | undefined, currency = 'CAD'): string {
  if (amount == null || !isFinite(amount)) return '—';
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

export function formatPercent(decimal: number | null | undefined): string {
  if (decimal == null || !isFinite(decimal)) return '—';
  const sign = decimal >= 0 ? '+' : '';
  return `${sign}${decimal.toFixed(2)}%`;
}

export function formatNumber(n: number | null | undefined, decimals = 2): string {
  if (n == null || !isFinite(n)) return '—';
  return new Intl.NumberFormat('en-CA', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(n);
}

export function formatCompact(n: number | null | undefined): string {
  if (n == null || !isFinite(n)) return '—';
  const abs = Math.abs(n);
  const sign = n < 0 ? '-' : '';
  if (abs >= 1_000_000) return `${sign}$${(abs / 1_000_000).toFixed(1)}M`;
  if (abs >= 1_000) return `${sign}$${(abs / 1_000).toFixed(1)}K`;
  return `${sign}$${abs.toFixed(2)}`;
}
