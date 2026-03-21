/**
 * Centralized frontend configuration.
 *
 * Runtime/tunable settings live here. Static domain data (preset scenarios,
 * asset type metadata, chart range options) stays in constants.ts.
 */
export const config = {
  // ── Currency ────────────────────────────────────────────────────────────────
  /** Fallback base currency when no user preference is stored. */
  defaultBaseCurrency: 'CAD',

  // ── UI counts ────────────────────────────────────────────────────────────────
  /** Number of top movers shown in the Dashboard panel. */
  topMoversCount: 5,

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
