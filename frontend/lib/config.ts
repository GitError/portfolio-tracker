/**
 * Centralized frontend configuration.
 *
 * Runtime/tunable settings live here. Static domain data (preset scenarios,
 * asset type metadata, chart range options) stays in constants.ts.
 */

/**
 * Page size used when the caller needs to load the full data set in one request
 * (e.g. holdings list, alerts list). Set high enough that it acts as a practical
 * "fetch all", but kept in one place so it can be tuned or replaced with true
 * cursor-based pagination in the future.
 */
export const PAGINATION_FETCH_ALL_SIZE = 500;

export const config = {
  // ── Currency ────────────────────────────────────────────────────────────────
  /** Fallback base currency when no user preference is stored. */
  defaultBaseCurrency: 'CAD',

  // ── UI counts ────────────────────────────────────────────────────────────────
  /** Number of top movers shown in the Dashboard panel. */
  topMoversCount: 10,

  /** Minimum symbol length before SymbolSearch fires a query. */
  symbolSearchMinChars: 2,

  // ── Timing ───────────────────────────────────────────────────────────────────
  /** Debounce delay (ms) for symbol search input. */
  symbolSearchDebounceMs: 300,

  /** Debounce delay (ms) for stress-test slider changes. */
  stressTestDebounceMs: 150,

  /** Auto-dismiss delay (ms) for toast notifications. */
  toastDismissMs: 4000,
} as const;
