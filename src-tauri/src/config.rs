//! Centralized configuration constants for the portfolio tracker backend.
//!
//! All hardcoded values that may need tuning or are referenced in multiple
//! places should live here, not inline in the modules that use them.

// ── Currency ──────────────────────────────────────────────────────────────────

/// Default base currency used for all portfolio valuations.
/// In the future this will become a runtime user setting stored in the DB.
pub const BASE_CURRENCY: &str = "CAD";

// ── External APIs ─────────────────────────────────────────────────────────────

pub const YAHOO_CHART_URL: &str =
    "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=1d";

pub const YAHOO_QUOTE_URL: &str = "https://query1.finance.yahoo.com/v7/finance/quote?symbols={}";

/// Endpoint hit once to obtain the session cookie Yahoo Finance requires
/// before it will issue a crumb token. Any Yahoo Finance host works; this
/// one returns quickly and needs no query parameters.
pub const YAHOO_COOKIE_URL: &str = "https://fc.yahoo.com";

/// Exchanges the session cookie from [`YAHOO_COOKIE_URL`] for a crumb token.
/// The v7 quote endpoint has required a matching cookie + crumb pair since
/// Yahoo tightened access to it (see #789); the v8 chart endpoint above is
/// unaffected.
pub const YAHOO_CRUMB_URL: &str = "https://query1.finance.yahoo.com/v1/test/getcrumb";

/// Endpoint for per-symbol fundamental metadata (sector, industry, country).
/// Replace `{}` with the symbol. Returns `quoteSummary.result[0].assetProfile`.
pub const YAHOO_QUOTE_SUMMARY_URL: &str =
    "https://query2.finance.yahoo.com/v11/finance/quoteSummary/{}?modules=assetProfile";

/// User-Agent sent with every outbound HTTP request.
/// Yahoo Finance returns 403 without a browser-like UA string.
pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)";

// ── Database ──────────────────────────────────────────────────────────────────

pub const DB_FILE_NAME: &str = "portfolio.db";

// ── Import limits ─────────────────────────────────────────────────────────────

/// Maximum number of rows accepted in a single CSV import.
pub const MAX_IMPORT_ROWS: usize = 500;

/// Maximum length (in bytes) for any individual string field in a CSV import row.
pub const MAX_FIELD_LEN: usize = 500;

/// Maximum size (in bytes) of a file accepted by the `parse_import_file` pipeline.
/// Guards against loading an excessively large or malicious file fully into memory.
pub const MAX_IMPORT_FILE_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB

// ── Cache TTLs ────────────────────────────────────────────────────────────────

/// How long (seconds) a symbol search result is cached in memory and SQLite.
/// Symbol names and exchange listings change rarely; 5 minutes balances freshness with performance.
pub const SEARCH_CACHE_TTL_SECS: i64 = 300; // 5 minutes

/// Maximum number of entries to keep in the in-memory search cache.
/// 200 entries covers typical portfolios (10–50 symbols) with generous room for exploratory searches.
pub const SEARCH_CACHE_MAX_ENTRIES: usize = 200;

/// Maximum length (in characters) for a symbol search query string.
/// Queries longer than this are rejected without hitting the network or cache.
pub const MAX_SEARCH_QUERY_LEN: usize = 100;
