import type { Holding, PortfolioSnapshot } from '../types/portfolio';

const CACHE_KEY = 'portfolio_snapshot_cache';

interface PortfolioCache {
  snapshot: PortfolioSnapshot;
  holdings: Holding[];
}

/** Lightweight schema validation for the localStorage cache to prevent loading corrupt data. */
function isValidPortfolioCache(value: unknown): value is PortfolioCache {
  if (!value || typeof value !== 'object') return false;
  const obj = value as Record<string, unknown>;
  if (!obj.snapshot || typeof obj.snapshot !== 'object') return false;
  const snap = obj.snapshot as Record<string, unknown>;
  if (!Array.isArray(snap.holdings)) return false;
  if (typeof snap.totalValue !== 'number') return false;
  if (typeof snap.lastUpdated !== 'string') return false;
  if (!Array.isArray(obj.holdings)) return false;
  return true;
}

/** Load the last-known portfolio snapshot and holdings list from localStorage. Returns null on miss or corrupt data. */
export function loadCachedPortfolio(): PortfolioCache | null {
  try {
    const raw = localStorage.getItem(CACHE_KEY);
    if (!raw) return null;
    const parsed: unknown = JSON.parse(raw);
    if (!isValidPortfolioCache(parsed)) {
      localStorage.removeItem(CACHE_KEY);
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

/** Persist the current portfolio snapshot and holdings list to localStorage. Best-effort — silently ignores storage quota errors. */
export function saveCachedPortfolio(snapshot: PortfolioSnapshot, holdings: Holding[]): void {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify({ snapshot, holdings }));
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
