# Roadmap

Planned improvements to Portfolio Tracker, organized by horizon. Status markers:

- ✅ Done
- 🚧 In Progress
- 🔲 Planned

---

## Near-term (v1.x)

Incremental improvements to the existing feature set.

| Status | Feature | Description |
|--------|---------|-------------|
| ✅ | CSV Import / Export | Bulk-import holdings from a CSV file with symbol validation and preview. Export back to CSV at any time. |
| ✅ | Historical Snapshots | Portfolio value recorded to SQLite on every price refresh. Performance view shows real data. |
| ✅ | Benchmark Overlay | Overlay S&P 500, NASDAQ 100, TSX, or Bitcoin on the Performance chart as a reference line. |
| ✅ | Price Alerts | Set above/below price threshold alerts per symbol; triggered automatically on each refresh. |
| ✅ | Account Types | Tag holdings as TFSA, RRSP, Taxable, or Cash; filter the Holdings table by account. |
| ✅ | Rebalancing | Set target allocation weights per holding; view drift, required trades, and deployable cash guidance. |
| ✅ | Dividend Tracking | Record dividend payments with ex-date and pay date; view payout history and totals by symbol. |
| ✅ | Settings Panel | Configurable base currency, auto-refresh interval, and cost basis method. |
| ✅ | Configurable Base Currency | Display all values in CAD, USD, EUR, GBP, AUD, CHF, or JPY. |
| ✅ | Auto-refresh | Background price refresh on a configurable interval (1m–1hr) with TopBar countdown. |
| ✅ | Symbol Search | Live symbol autocomplete via Yahoo Finance with local caching. |
| ✅ | Keyboard Shortcuts | Full keyboard navigation; `?` to see all shortcuts. |
| ✅ | JSON Backup / Restore | Export and import all data — holdings, alerts, transactions, dividends, and config. |
| ✅ | In-app Alert Notifications | Toast notifications when price alerts fire during auto-refresh. |
| ✅ | Transaction History | Per-holding buy/sell log; drives AVCO and FIFO cost basis calculations. |
| ✅ | Analytics | Sector breakdown, country exposure, weighted beta, P/E, dividend yield, realized gains, and HHI concentration. |
| ✅ | Accounts Modal | Named account management (TFSA, RRSP, FHSA, Taxable, Crypto, Other). |
| ✅ | Action Center | Quick-access side panel for alert triggers and fast transaction entry. |
| ✅ | Annual Dividend Income | Dashboard card shows trailing 12-month dividend income from recorded payment events. |
| ✅ | SQLx Migration | Database layer migrated from rusqlite to SQLx with async connection pool and WAL mode. |
| ✅ | Dark / Light Theme Toggle | Light theme variant selectable in Settings. |
| ✅ | i18n / Multi-language | Language picker in Settings with i18next-based translations; Analytics, Alerts, Settings, and Dividends now fully wired to `t()` alongside Dashboard/TopBar. |
| ✅ | Dependency Maintenance Refresh | May 2026 Dependabot updates merged for React ecosystem, Tauri packages, Tokio, uuid, i18next, lucide-react, and dev tooling. A further July 2026 round merged 9 more PRs (chrono, serde, sqlx, tauri-build, lucide-react, recharts, i18next, uuid, @tauri-apps/api, dev-dependencies group, tauri-action, checkout). |
| ✅ | CI/Security Regression Fixes | July 2026 — a later dependency merge had reintroduced nonexistent `actions/checkout@v6`/`setup-node@v6` and a disabled CSP; both re-fixed, plus the theme/language flash-of-wrong-preference bug on Tauri launch. See `docs/analysis-2026-03-22.md` for the full verified fix list. |

---

## Next Up: Guided Import + Insights

Design finalized, not yet implemented — see [`docs/superpowers/specs/2026-05-24-import-plus-insights-design.md`](superpowers/specs/2026-05-24-import-plus-insights-design.md) for the full spec. This is the next major feature and should be the default starting point for new feature work.

Replaces the current strict, CSV-only, canonical-header importer with a guided wizard: upload CSV or XLSX → pick account context once → deterministic column inference against a broker-alias registry → reviewable import plan (`create`/`update`/`skip`/`needs_fix`/`warning` per row) → commit clean rows → post-import insight panel (new positions, drift from target weights, stale symbols, cash balance changes). Explicitly out of scope for v1: brokerage API integration, LLM-based column inference, and full transaction-history reconstruction.

---

## Medium-term (v2.0)

Larger features that extend the core model.

| Status | Feature | Description |
|--------|---------|-------------|
| 🔲 | Brokerage API Integration | Pull holdings and transactions directly from Questrade or Interactive Brokers via their APIs. Eliminates manual entry. |
| 🔲 | Options Tracking | Track basic options positions: symbol, strike, expiry, premium paid. P&L calculated at expiry or mark-to-market via Yahoo. |
| 🔲 | Monte Carlo Simulation | Run thousands of randomized future-price paths based on historical volatility. Displays a probability cone over a chosen time horizon. |
| 🔲 | Historical Scenario Replay | Apply shocks derived from real historical events — 2008 financial crisis, COVID crash (Mar 2020), 2022 rate-hike cycle — to your current portfolio. |
| 🔲 | Tax Lot Tracking | Record individual buy lots, apply ACB (adjusted cost base) methodology for Canadian capital gains calculations. Fulfils the FIFO/AVCO setting already in Settings. |
| 🔲 | Export to PDF | Generate a portfolio summary PDF for tax or record-keeping purposes. |

---

## Long-term / Exploratory

Features that require significant architectural work or are still being evaluated.

| Status | Feature | Description |
|--------|---------|-------------|
| 🔲 | Mobile Companion | A read-only mobile app (Tauri mobile or React Native) that syncs with the desktop database via iCloud or a local network connection. |
| 🔲 | Multi-Portfolio Support | Separate portfolios per account type (RRSP, TFSA, taxable) with independent performance tracking. Account types already exist; this adds separate portfolio-level analytics. |
| 🔲 | AI-Powered Insights | Natural-language analysis of concentration risk, sector exposure, and rebalancing suggestions. |

---

## Recently Shipped

| Version | Feature |
|---------|---------|
| Unreleased | CI/CSP/theme-flash regression fixes (PR #575), remaining i18n wiring (Analytics/Alerts/Settings/Dividends), 9 more Dependabot merges — not yet tagged as a new version |
| v0.1.0-8 | Dependency maintenance refresh; all open dependency PRs merged and frontend lint config aligned with updated React Hooks plugin |
| v0.1.0-4 | SQLx migration (async pool + WAL mode), `src/` → `frontend/` rename, export/import extended to include transactions and dividends |
| v0.1.0-3 | Annual dividend income in Dashboard, backend hardening, analytics and performance fixes |
| v0.1.0-2 | Transaction History, Analytics, Accounts modal, Action Center, in-app alert toast notifications, full backup/restore |
| v0.2.0 | CSV import/export, historical snapshots, price alerts, account types, rebalancing, dividend tracking, settings panel, configurable base currency, auto-refresh, symbol search, keyboard shortcuts, JSON backup/restore |
| v0.1.0 | Initial release: Dashboard, Holdings, Performance, Stress Test, multi-currency FX, local SQLite persistence |
