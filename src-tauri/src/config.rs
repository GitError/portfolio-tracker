//! Centralized configuration constants for the portfolio tracker backend.
//!
//! All hardcoded values that may need tuning or are referenced in multiple
//! places should live here, not inline in the modules that use them.

// ── Currency ──────────────────────────────────────────────────────────────────

/// Default base currency used for all portfolio valuations.
/// In the future this will become a runtime user setting stored in the DB.
pub const BASE_CURRENCY: &str = "CAD";

/// All currencies the app can hold positions in / fetch FX rates for.
#[expect(
    dead_code,
    reason = "Centralized for the config refactor; not all call sites use it yet"
)]
pub const SUPPORTED_CURRENCIES: &[&str] = &["CAD", "USD", "EUR", "GBP", "JPY", "CHF", "AUD"];

// ── External APIs ─────────────────────────────────────────────────────────────

pub const YAHOO_CHART_URL: &str =
    "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=1d";

pub const YAHOO_SEARCH_URL: &str =
    "https://query1.finance.yahoo.com/v1/finance/search?q={}&quotesCount=8&newsCount=0&enableFuzzyQuery=false";

/// User-Agent sent with every outbound HTTP request.
/// Yahoo Finance returns 403 without a browser-like UA string.
pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)";

// ── Database ──────────────────────────────────────────────────────────────────

pub const DB_FILE_NAME: &str = "portfolio.db";

// ── Cache TTLs ────────────────────────────────────────────────────────────────

/// How long (seconds) a cached price is considered fresh before re-fetching.
#[expect(
    dead_code,
    reason = "Cache freshness logic is being centralized incrementally"
)]
pub const PRICE_CACHE_TTL_SECS: i64 = 300; // 5 minutes

/// How long (seconds) a cached FX rate is considered fresh.
#[expect(
    dead_code,
    reason = "Cache freshness logic is being centralized incrementally"
)]
pub const FX_CACHE_TTL_SECS: i64 = 900; // 15 minutes

/// How long (seconds) a symbol search result is cached.
pub const SEARCH_CACHE_TTL_SECS: i64 = 300; // 5 minutes

/// Maximum number of entries to keep in the in-memory search cache.
pub const SEARCH_CACHE_MAX_ENTRIES: usize = 200;
