import type { PortfolioSnapshot } from '../types/portfolio';

const CACHE_KEY = 'portfolio_snapshot_cache';

/**
 * Minimal offline fallback persisted to localStorage — totals only, never the
 * per-holding array (symbols, quantities, cost basis). See docs/privacy.md.
 */
export interface OfflineSnapshotCache {
  totalValue: number;
  holdingCount: number;
  lastUpdated: string;
  baseCurrency: string;
}

/** Lightweight schema validation for the localStorage cache to prevent loading corrupt data. */
function isValidOfflineSnapshotCache(value: unknown): value is OfflineSnapshotCache {
  if (!value || typeof value !== 'object') return false;
  const obj = value as Record<string, unknown>;
  return (
    typeof obj.totalValue === 'number' &&
    typeof obj.holdingCount === 'number' &&
    typeof obj.lastUpdated === 'string' &&
    typeof obj.baseCurrency === 'string'
  );
}

/** Load the last-known offline snapshot from localStorage. Returns null on miss or corrupt data. */
export function loadCachedPortfolio(): OfflineSnapshotCache | null {
  try {
    const raw = localStorage.getItem(CACHE_KEY);
    if (!raw) return null;
    const parsed: unknown = JSON.parse(raw);
    if (!isValidOfflineSnapshotCache(parsed)) {
      localStorage.removeItem(CACHE_KEY);
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

/** Persist a minimal offline snapshot (totals only) to localStorage. Best-effort — silently ignores storage quota errors. */
export function saveCachedPortfolio(snapshot: PortfolioSnapshot): void {
  try {
    const cache: OfflineSnapshotCache = {
      totalValue: snapshot.totalValue,
      holdingCount: snapshot.holdings.length,
      lastUpdated: snapshot.lastUpdated,
      baseCurrency: snapshot.baseCurrency,
    };
    localStorage.setItem(CACHE_KEY, JSON.stringify(cache));
  } catch {
    /* storage may be full — best effort */
  }
}

/** Remove the cached portfolio data from localStorage. */
export function clearSnapshotCache(): void {
  try {
    localStorage.removeItem(CACHE_KEY);
  } catch {
    /* best effort */
  }
}
