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
| 🔲 | CSV Import | Bulk-import holdings from a CSV file (symbol, quantity, cost basis, currency). Useful for migrating from a spreadsheet. |
| 🔲 | Historical Snapshots | Record daily portfolio values to SQLite so the Performance view shows real historical data instead of simulated data. |
| 🔲 | Benchmark Overlay | Overlay S&P 500 (^GSPC) or TSX (^GSPTSE) on the Performance chart as a reference line. |
| 🔲 | Portfolio Alerts | Notify when a holding crosses a price threshold or daily P&L exceeds a set amount. Uses macOS native notifications via Tauri. |
| 🔲 | Dark / Light Theme Toggle | Add a light theme variant. The current terminal-dark theme remains the default. |
| 🚧 | Sidebar Toggle | Replace hover-expand sidebar with a pinned toggle button (⌘B). ([#13](https://github.com/GitError/portfolio-tracker/issues/13)) |

---

## Medium-term (v2.0)

Larger features that extend the core model.

| Status | Feature | Description |
|--------|---------|-------------|
| 🔲 | Brokerage API Integration | Pull holdings and transactions directly from Questrade or Interactive Brokers via their APIs. Eliminates manual entry. |
| 🔲 | Options Tracking | Track basic options positions: symbol, strike, expiry, premium paid. P&L calculated at expiry or mark-to-market via Yahoo. |
| 🔲 | Monte Carlo Simulation | Run thousands of randomized future-price paths based on historical volatility. Displays a probability cone over a chosen time horizon. |
| 🔲 | Historical Scenario Replay | Apply shocks derived from real historical events — 2008 financial crisis, COVID crash (Mar 2020), 2022 rate-hike cycle — to your current portfolio. |
| 🔲 | Export to PDF / CSV | Generate a portfolio summary PDF or export all holdings and performance data to CSV for tax or record-keeping purposes. |

---

## Long-term / Exploratory

Features that require significant architectural work or are still being evaluated.

| Status | Feature | Description |
|--------|---------|-------------|
| 🔲 | Mobile Companion | A read-only mobile app (Tauri mobile or React Native) that syncs with the desktop database via iCloud or a local network connection. |
| 🔲 | Multi-Portfolio Support | Separate portfolios for registered accounts (RRSP, TFSA) and taxable accounts, each with independent performance tracking. |
| 🔲 | Tax Lot Tracking | Record individual buy lots, apply ACB (adjusted cost base) methodology for Canadian capital gains calculations. |
| 🔲 | AI-Powered Insights | Natural-language analysis of concentration risk, sector exposure, and rebalancing suggestions ("your portfolio is 60% US tech"). |

---

## Recently Shipped

| Version | Feature |
|---------|---------|
| v0.1.0 | Initial release: Dashboard, Holdings, Performance, Stress Test, multi-currency FX |
| v0.1.1 | Holdings persistence fix — data now survives app restarts ([#12](https://github.com/GitError/portfolio-tracker/issues/12)) |
