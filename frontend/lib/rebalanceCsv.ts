import type { RebalanceSuggestion } from '../types/portfolio';

/**
 * Neutralizes CSV/spreadsheet formula injection: values starting with `=`, `+`,
 * `-`, `@`, tab, or carriage return are interpreted as formulas by
 * Excel/LibreOffice/Google Sheets when the exported file is opened. Prefixing
 * with a single quote forces the cell to be treated as literal text instead.
 * Mirrors `neutralize_formula_injection` in src-tauri/src/csv.rs.
 */
function neutralizeFormulaInjection(value: string): string {
  return /^[=+\-@\t\r]/.test(value) ? `'${value}` : value;
}

export function buildCsvContent(suggestions: RebalanceSuggestion[]): string {
  const header = [
    'symbol',
    'name',
    'current_weight_%',
    'target_weight_%',
    'drift_pp',
    'action',
    'units',
    'amount_cad',
    'current_price_cad',
  ].join(',');

  const rows = suggestions.map((s) => {
    const action = s.suggestedTradeCad > 0 ? 'sell' : 'buy';
    const units = Math.abs(s.suggestedUnits).toFixed(4);
    const amount = Math.abs(s.suggestedTradeCad).toFixed(2);
    const name = neutralizeFormulaInjection(s.name);
    return [
      neutralizeFormulaInjection(s.symbol),
      `"${name.replace(/"/g, '""')}"`,
      s.currentWeight.toFixed(2),
      s.targetWeight.toFixed(2),
      s.drift.toFixed(2),
      action,
      units,
      amount,
      s.currentPriceCad.toFixed(4),
    ].join(',');
  });

  return [header, ...rows].join('\n');
}
