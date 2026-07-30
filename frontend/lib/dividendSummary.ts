import type { Dividend, Holding } from '../types/portfolio';

export interface DividendSummaryEntry {
  total: number;
  currency: string;
  count: number;
}

/**
 * Aggregates recorded dividends by symbol into total cash received (amountPerUnit x quantity).
 *
 * LIMITATION (#602): uses each holding's *current* quantity, not the quantity actually held on
 * the dividend's pay date. `Dividend` records (types/bindings/Dividend.ts, mirrored from the
 * Rust struct in src-tauri/src/types.rs) don't capture a per-payment quantity/shares snapshot,
 * so there is nothing to reconstruct an accurate historical total from. If shares were bought or
 * sold after a dividend was paid, the total shown for that symbol will not match the amount
 * actually received. Fixing this properly requires capturing quantity on the Dividend record at
 * entry time (a schema change) — tracked as a follow-up, not attempted here to avoid inventing
 * data (e.g. reconstructing from transaction history) that may be incomplete or wrong for
 * holdings without full transaction history, such as CSV-imported ones.
 */
export function summarizeDividendsBySymbol(
  dividends: Dividend[],
  holdings: Holding[]
): Record<string, DividendSummaryEntry> {
  const bySymbol: Record<string, DividendSummaryEntry> = {};
  for (const div of dividends) {
    if (!bySymbol[div.symbol]) {
      bySymbol[div.symbol] = { total: 0, currency: div.currency, count: 0 };
    }
    const holding = holdings.find((h) => h.id === div.holdingId);
    const entry = bySymbol[div.symbol]!;
    entry.total += div.amountPerUnit * (holding?.quantity ?? 0);
    entry.count += 1;
  }
  return bySymbol;
}
