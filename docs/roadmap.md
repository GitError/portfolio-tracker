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
| ✅ | JSON Backup / Restore | Export and import all holdings as JSON. |
| 🔲 | In-app Alert Notifications | Show a toast / notification when a price alert fires during auto-refresh (see [#158](https://github.com/GitError/portfolio-tracker/issues/158)). |
| 🔲 | Full Backup / Restore | Extend export/import to include alerts, dividends, and settings — not just holdings (see [#159](https://github.com/GitError/portfolio-tracker/issues/159)). |
| 🔲 | Dark / Light Theme Toggle | Add a light theme variant. The current terminal-dark theme remains the default. |

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
| v0.1.x | CSV import/export, historical snapshots, benchmark overlay, price alerts, account types, rebalancing with target weights, dividend tracking, settings panel, configurable base currency, auto-refresh, symbol search, keyboard shortcuts, JSON backup/restore |
| v0.1.0 | Initial release: Dashboard, Holdings, Performance, Stress Test, multi-currency FX, local SQLite persistence |
