import type { RowAction } from '../../types/portfolio';

export const MONO: React.CSSProperties = { fontFamily: 'var(--font-mono)' };

export const TD: React.CSSProperties = {
  padding: '6px 10px',
  borderBottom: '1px solid var(--border-subtle)',
  verticalAlign: 'middle',
};

export const STATUS_META: Record<RowAction, { i18nKey: string; color: string }> = {
  create: { i18nKey: 'importWizard.status.create', color: 'var(--color-gain)' },
  update: { i18nKey: 'importWizard.status.update', color: 'var(--color-accent)' },
  skip: { i18nKey: 'importWizard.status.skip', color: 'var(--text-muted)' },
  needs_fix: { i18nKey: 'importWizard.status.needsFix', color: 'var(--color-crypto)' },
  warning: { i18nKey: 'importWizard.status.warning', color: 'var(--color-warning)' },
};

/** Canonical holding fields the backend's alias registry knows how to fill (see
 * `import_pipeline::aliases::canonical_field`). Used to populate the manual
 * column-mapping dropdown. */
export const CANONICAL_FIELDS: { value: string; i18nKey: string }[] = [
  { value: 'symbol', i18nKey: 'importWizard.field.symbol' },
  { value: 'name', i18nKey: 'importWizard.field.name' },
  { value: 'quantity', i18nKey: 'importWizard.field.quantity' },
  { value: 'average_cost', i18nKey: 'importWizard.field.averageCost' },
  { value: 'book_value', i18nKey: 'importWizard.field.bookValue' },
  { value: 'currency', i18nKey: 'importWizard.field.currency' },
  { value: 'market_value', i18nKey: 'importWizard.field.marketValue' },
  { value: 'asset_type', i18nKey: 'importWizard.field.assetType' },
  { value: 'exchange', i18nKey: 'importWizard.field.exchange' },
  { value: 'target_weight', i18nKey: 'importWizard.field.targetWeight' },
  { value: 'cash_balance', i18nKey: 'importWizard.field.cashBalance' },
  { value: 'dividend_yield', i18nKey: 'importWizard.field.dividendYield' },
  { value: 'annualized_income', i18nKey: 'importWizard.field.annualizedIncome' },
  { value: 'ex_dividend_date', i18nKey: 'importWizard.field.exDividendDate' },
];

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** Non-standard Tauri v2 property present on File objects sourced from the webview
 * (both <input type="file"> selection and drag-drop), giving the real filesystem path. */
export function extractFilePath(file: File): string | undefined {
  return (file as File & { path?: string }).path;
}
