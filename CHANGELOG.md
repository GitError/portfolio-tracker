# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
