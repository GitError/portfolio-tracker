# Roadmap

Planned improvements to Portfolio Tracker, organized by horizon. Status markers:

- ✅ Done
- 🚧 In Progress
- 🔲 Planned
- ❄️ Icebox (deferred, revisit later)

---

## Near-term (next up)

Financial correctness and data-trust features — making the numbers you see worth acting on.

| Status | Feature | Description |
|--------|---------|-------------|
| ✅ | Multi-currency realized gains | Realized gain/loss and proceeds converted to base currency per holding before aggregation. Mixed CAD/USD portfolios now show correct totals. PR #760. |
| 🔲 | Canadian tax-lot / ACB tracking | Record individual buy lots; apply Adjusted Cost Base methodology with a documented FX convention for Canadian capital gains calculations. |
| 🔲 | Corporate actions & DRIP support | Handle splits, mergers/spinoffs, and dividend reinvestments as first-class events — prerequisite for durable historic cost basis. |
| 🔲 | Performance methodology | Define and implement Time-Weighted Return (TWR) and/or Money-Weighted Return (XIRR) so the Performance view shows a methodology-correct return, not just a snapshot delta. |
| 🔲 | Reconciliation-friendly broker imports | Extend the Import wizard to match imported rows against existing lots, flag discrepancies, and suggest reconciliations. File-first before any live API integration. |

---

## Medium-term

Larger features that extend the core model.

| Status | Feature | Description |
|--------|---------|-------------|
| ✅ | Historical Scenario Replay | Apply shocks derived from real historical events — 2008 financial crisis, COVID crash (Mar 2020), 2022 rate-hike cycle — to your current portfolio. PR #753. |
| ✅ | Export to PDF | Generate a portfolio summary PDF (holdings table, allocation breakdown) auto-saved to `~/Downloads/portfolio-YYYY-MM-DD.pdf`. PR #765. |
| 🔲 | Tax Lot Tracking | Record individual buy lots, apply ACB (adjusted cost base) methodology for Canadian capital gains calculations. |
| 🔲 | Multi-portfolio support | Separate portfolios by person or goal with independent performance tracking — distinct from the existing account-type model. |
| ❄️ | Brokerage API Integration | Pull holdings and transactions directly from Questrade or Interactive Brokers. Deferred: solve credentials, refresh, reconciliation, and sync deliberately before going live. |
| ❄️ | Options Tracking | Track basic options positions: symbol, strike, expiry, premium paid. Deferred: not a small extension to equities/ETFs — warrants a separate module. |
| ❄️ | Monte Carlo Simulation | Randomized future-price paths based on historical volatility. Deferred until correctness work is solid — stress testing is more immediately actionable. |

---

## Long-term / Exploratory

| Status | Feature | Description |
|--------|---------|-------------|
| ❄️ | Mobile Companion | A read-only mobile view that syncs with the desktop database. Blocked on a secure sync/key-management model. |
| ❄️ | AI-Powered Insights | Natural-language analysis of concentration risk, sector exposure, and rebalancing suggestions. MCP already enables intentional external-assistant analysis; avoid embedding a generic chatbot. |

---

## Recently Shipped

| Version | Feature |
|---------|---------|
| post-v0.2.0 | Export to PDF (#765), MCP write opt-in (#762), localStorage privacy + threat model (#763), centralize shared validation in portfolio-core (#764), multi-currency realized gains (#760), CI workspace coverage for portfolio-core/portfolio-mcp (#761), Historical Scenario Replay presets (#753) |
| v0.2.0 | Import Plus Insights wizard (CSV/XLSX → broker-alias inference → import plan → post-import insights), XLSX import (`calamine`), CRA T5/T5008 XML import (`quick-xml`), analytics N+1 fix, i18n completion, CI/CSP/theme-flash regression fixes, 9 Dependabot PRs |
| v0.1.0-9 | 2026-08-02 housekeeping pass: `portfolio-core` shared crate, MCP expansion, atomic backup/restore, WAL checkpoint, config validation, React ErrorBoundary, locale-aware formatting |
| v0.1.0-4 | SQLx migration, `src/` → `frontend/` rename, export/import extended to transactions and dividends |
| v0.1.0-3 | Annual dividend income, Analytics, Transaction History, Accounts modal, Action Center |
| v0.1.0 | Initial release: Dashboard, Holdings, Performance, Stress Test, multi-currency FX, local SQLite |
