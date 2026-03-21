# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-4] - 2026-03-21

### Added
- **Annual Dividend Income** — Dashboard card shows trailing 12-month dividend income from recorded payment events; FX-converted to base currency per dividend

### Changed
- **Database layer migrated to SQLx** — replaced rusqlite / `Mutex<Connection>` with an async `SqlitePool` (WAL mode, 5-second busy timeout, FK enforcement); all DB functions are now `async fn`; schema managed via `src-tauri/migrations/`
- **Frontend directory renamed** — `src/` → `frontend/` for clearer project structure; all config files (vite, tsconfig, vitest, eslint, package.json, CI) updated accordingly
- **Export/Import extended** — JSON export/import now includes transactions and dividends in addition to holdings, alerts, and config
- **`isTauri()` guard** — all `invoke()` calls wrapped with an `isTauri()` check so the frontend works in browser dev mode
- **Static benchmark series removed** — Performance chart benchmark overlay now fetches live data on demand only

### Fixed
- Async deadlock in `get_base_currency` — removed `block_on` inside async command, now properly `await`ed
- LRU eviction in symbol search cache — evicts oldest entry instead of clearing the entire cache
- `cost_basis_method` lowercased before storing to prevent case-mismatch bugs
- `import_data` DELETE operations run inside a single SQLx transaction for atomicity
- Snapshot retention extended from 30 days to 730 days (2 years of daily snapshots)
- `delete_account` fixed to guard by UUID instead of account type string
- `busy_timeout(5000ms)` set on the SQLx pool to prevent immediate lock errors under concurrent access
- Dividend FX conversion logs a warning when the required rate is missing

## [0.1.0-3] - 2026-03-18

### Added
- **Analytics view** — sector breakdown donut, country exposure, weighted beta, P/E, dividend yield, realized gains, and HHI portfolio concentration score
- **Transaction History view** — per-holding buy/sell log; feeds AVCO and FIFO cost basis calculations
- **Accounts modal** — manage named accounts (TFSA, RRSP, FHSA, Taxable, Crypto, Other); holdings assigned to accounts for account-level filtering
- **Action Center** — quick-access side panel surfacing recent alert triggers and a fast transaction entry form
- **Help screen** — keyboard shortcuts reference accessible via `?` key; also accessible from the sidebar

### Fixed
- In-app toast notifications when price alerts trigger during auto-refresh
- Dashboard chart overflow clipped; Action Center collapsed state persisted correctly; account selector z-index corrected
- Custom Select component replaces all native `<select>` elements; date inputs styled consistently
- Analytics sector data sourced from `quoteSummary/assetProfile`; geographic chart hidden when data is too sparse
- FK constraint on transaction inserts; CSV import account fallback; alert symbol dropdown fixed
- Portfolio Value card whitespace; Top Movers count display; Holdings toolbar alignment
- Backup/restore extended to include alerts and config (previously holdings only)
- Price currency fallback when Yahoo Finance omits the `currency` field
- AVCO calculation corrected for partial-sell lot matching
- Alert errors surfaced in TopBar after each refresh
- Daily P&L and performance chart panic guards added for empty price history
- `delete_account` column reference fixed; `init_db` migration guard added; FX rate inversion bug fixed

## [0.2.0] - 2026-03-15

### Added
- **Rebalancing view** — set target allocation weights per holding; view drift, required trade sizes, and deployable cash guidance
- **Price alerts** — set above/below price threshold alerts per symbol; alerts are checked on every price refresh and flagged in the Alerts view
- **Dividend tracking** — record dividend payments per unit with ex-date and pay date; summary grid and full history table
- **Settings panel** — configurable base currency (CAD, USD, EUR, GBP, AUD, CHF, JPY), auto-refresh interval (1m–1hr), and cost basis method (AVCO / FIFO)
- **Auto-refresh** — background price refresh on a configurable interval with a TopBar countdown timer
- **Configurable base currency** — all portfolio values convert to the selected base currency; changing currency immediately triggers a price refresh
- **Symbol search autocomplete** — live Yahoo Finance symbol search with local SQLite caching for fast repeat lookups
- **CSV import** — bulk-import holdings from a CSV file with symbol validation, preview screen (shows ready/duplicate/invalid status per row), and `SYMBOL:COUNTRY` notation support (e.g. `BMO:CA` → `BMO.TO`)
- **CSV export** — export all holdings to CSV (`⌘E`); re-importable format
- **Account types** — tag holdings as TFSA, RRSP, Taxable, or Cash; filter the Holdings table by account
- **Benchmark overlay** — overlay S&P 500, NASDAQ 100, TSX, or Bitcoin on the Performance area chart
- **Real performance snapshots** — portfolio value recorded to SQLite on every price refresh; Performance view now shows real historical data
- **Keyboard shortcuts** — `⌘N` add holding, `⌘R` refresh, `⌘E` export CSV, `⌘,` settings, `1–4` navigate views, `?` shortcuts overlay
- **JSON backup / restore** — export all holdings as JSON; import replaces current holdings
- **Shared portfolio context** — holdings changes (add, edit, delete, import) immediately refresh all views (Dashboard, Performance, Stress Test)
- **Failed-symbol warning banner** — TopBar shows which symbols failed to refresh with a one-click retry

### Changed
- TopBar now shows base currency picker and auto-refresh countdown alongside the refresh button
- Holdings table gains Account and Exchange columns; sortable and filterable
- Stress Test presets expanded from 5 to 11 scenarios; preset names adjust dynamically to the selected base currency

## [0.1.0] - 2026-03-14

### Added
- Dashboard with portfolio value, allocation charts, and top movers
- Holdings CRUD for stocks, ETFs, crypto, and cash
- Historical performance charts and portfolio stats
- Stress testing with preset and custom scenarios
- Live Yahoo Finance pricing and FX conversion
- Local SQLite persistence via Tauri and Rust
