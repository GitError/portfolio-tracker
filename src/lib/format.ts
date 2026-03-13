export function formatCurrency(amount: number, currency = 'CAD'): string {
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
  const sign = decimal >= 0 ? '+' : '';
  return `${sign}${decimal.toFixed(2)}%`;
}

export function formatNumber(n: number, decimals = 2): string {
  return new Intl.NumberFormat('en-CA', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(n);
}

export function formatCompact(n: number): string {
  const abs = Math.abs(n);
  const sign = n < 0 ? '-' : '';
  if (abs >= 1_000_000) return `${sign}$${(abs / 1_000_000).toFixed(1)}M`;
  if (abs >= 1_000) return `${sign}$${(abs / 1_000).toFixed(1)}K`;
  return `${sign}$${abs.toFixed(2)}`;
}
