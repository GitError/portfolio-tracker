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
| ✅ | i18n / Multi-language | Language picker in Settings with i18next-based translations; all views fully wired to `t()`. |
| ✅ | Dependency Maintenance Refresh | Multiple rounds of Dependabot updates merged across the lifecycle. |
| ✅ | CI/Security Regression Fixes | CI workflow pinning, CSP policy, and theme-flash regression fixed across multiple releases. |
| ✅ | Shared `portfolio-core` Crate | Snapshot, FX, and stress-test math extracted into a shared workspace crate for `portfolio-mcp` parity. |
| ✅ | MCP Server Expansion | `portfolio-mcp` covers holdings, transactions, alerts, dividends, accounts, config, snapshots, and stress tests (20+ tools). |
| ✅ | Backend Hardening | Atomic backup/restore, WAL checkpoint task, bounded pool shutdown, `AppError::NotFound`/`Conflict`, per-key config validation. |
| ✅ | Comprehensive Locale-Aware Formatting | Percent, compact-currency, and target-weight formatting respect the active locale everywhere. |
| ✅ | Error Boundary | React error boundary wraps the app root; Alerts and Dividends keep a persistent error banner on load failure. |
| ✅ | Guided Import + Insights wizard | CSV/XLSX → column inference → reviewable import plan → commit → post-import insights. Replaces the old strict CSV importer. PR #745. |
| ✅ | XLSX import | Upload `.xlsx` files to the import wizard; parsed by the Rust backend using `calamine`. PR #747. |
| ✅ | CRA T5 / T5008 XML import | Parse CRA-issued tax return XML (T5008 dispositions, T5 income) for pre-population of the import wizard. PR #751. |
| ✅ | Analytics N+1 fix | Batched fundamentals cache lookup eliminates per-symbol queries in `useAnalytics`. PR #738. |

---

## Medium-term (v2.0)

Larger features that extend the core model.

| Status | Feature | Description |
|--------|---------|-------------|
| 🔲 | Brokerage API Integration | Pull holdings and transactions directly from Questrade or Interactive Brokers via their APIs. Eliminates manual entry. |
| 🔲 | Options Tracking | Track basic options positions: symbol, strike, expiry, premium paid. P&L calculated at expiry or mark-to-market via Yahoo. |
| 🔲 | Monte Carlo Simulation | Run thousands of randomized future-price paths based on historical volatility. Displays a probability cone over a chosen time horizon. |
| 🔲 | Historical Scenario Replay | Apply shocks derived from real historical events — 2008 financial crisis, COVID crash (Mar 2020), 2022 rate-hike cycle — to your current portfolio. |
| 🔲 | Tax Lot Tracking | Record individual buy lots, apply ACB (adjusted cost base) methodology for Canadian capital gains calculations. |
| 🔲 | Export to PDF | Generate a portfolio summary PDF for tax or record-keeping purposes. |

---

## Long-term / Exploratory

Features that require significant architectural work or are still being evaluated.

| Status | Feature | Description |
|--------|---------|-------------|
| 🔲 | Mobile Companion | A read-only mobile app (Tauri mobile or React Native) that syncs with the desktop database via iCloud or a local network connection. |
| 🔲 | Multi-Portfolio Support | Separate portfolios per account type (RRSP, TFSA, taxable) with independent performance tracking. |
| 🔲 | AI-Powered Insights | Natural-language analysis of concentration risk, sector exposure, and rebalancing suggestions. |

---

## Recently Shipped

| Version | Feature |
|---------|---------|
| v0.2.0 | Import Plus Insights wizard (CSV/XLSX → broker-alias inference → import plan → post-import insights), XLSX import (`calamine`), CRA T5/T5008 XML import (`quick-xml`), analytics N+1 fix, i18n completion, CI/CSP/theme-flash regression fixes, 9 Dependabot PRs |
| v0.1.0-9 | 2026-08-02 housekeeping pass (PRs #599–#715): `portfolio-core` shared crate, MCP account/dividend/update-holding tools + validation parity, atomic/concurrency-guarded backup-restore, WAL checkpoint task, config allowlist + per-key value validation, React `ErrorBoundary`, comprehensive locale-aware formatting — see `docs/releases.md` |
| v0.1.0-8 | Dependency maintenance refresh; all open dependency PRs merged and frontend lint config aligned with updated React Hooks plugin |
| v0.1.0-4 | SQLx migration (async pool + WAL mode), `src/` → `frontend/` rename, export/import extended to include transactions and dividends |
| v0.1.0-3 | Annual dividend income in Dashboard, backend hardening, analytics and performance fixes |
| v0.1.0-2 | Transaction History, Analytics, Accounts modal, Action Center, in-app alert toast notifications, full backup/restore |
| v0.1.0 | Initial release: Dashboard, Holdings, Performance, Stress Test, multi-currency FX, local SQLite persistence |
