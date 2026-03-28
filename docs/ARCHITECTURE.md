# Architecture

Technical reference for the Portfolio Tracker codebase.

---

## Tech Stack

| Layer     | Technology                                        |
|-----------|---------------------------------------------------|
| Shell     | [Tauri v2](https://tauri.app)                     |
| Backend   | Rust — tokio, reqwest, sqlx, chrono, uuid         |
| Frontend  | React 18 + TypeScript + Vite                      |
| Styling   | Tailwind CSS v4                                   |
| Charts    | [Recharts](https://recharts.org)                  |
| Icons     | [lucide-react](https://lucide.dev)                |
| Router    | react-router-dom v7                               |
| Database  | SQLite via SQLx (WAL mode, async connection pool) |

---

## Directory Tree

```
portfolio-tracker/
├── src-tauri/                   # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── migrations/              # SQLx migrations (numbered)
│   └── src/
│       ├── main.rs              # Tauri entry point
│       ├── lib.rs               # App bootstrap, state init, command registration
│       ├── config.rs            # App-level constants (DB name, user-agent, TTLs)
│       ├── types.rs             # All shared Rust types (serde camelCase)
│       ├── commands.rs          # Tauri command handlers (thin wrappers over domain logic)
│       ├── db.rs                # SQLite schema, migrations, CRUD (async SQLx)
│       ├── portfolio.rs         # build_portfolio_snapshot + helpers
│       ├── csv.rs               # CSV import/export helpers
│       ├── price.rs             # Yahoo Finance price fetching
│       ├── fx.rs                # FX rate fetching + conversion helpers
│       ├── search.rs            # Symbol search via Yahoo Finance
│       ├── analytics.rs         # Realized gains + portfolio analytics
│       └── stress.rs            # Stress test engine
└── frontend/                    # React frontend
    ├── App.tsx                  # Router, providers, keyboard shortcut wiring
    ├── main.tsx                 # React entry point
    ├── index.css                # Tailwind + global styles + design tokens
    ├── types/
    │   └── portfolio.ts         # TypeScript types (mirrors Rust structs exactly)
    ├── hooks/
    │   ├── usePortfolio.ts      # Tauri invoke wrapper + shared portfolio state
    │   ├── useStressTest.ts     # Stress test invocation + state
    │   ├── useConfig.ts         # Persistent key/value config via Tauri
    │   ├── useAutoRefresh.ts    # Interval-based auto price refresh + countdown
    │   └── useKeyboardShortcuts.ts  # Global keyboard shortcut handler
    ├── lib/
    │   ├── format.ts            # Currency/number/percent formatters
    │   ├── colors.ts            # PnL color helpers
    │   ├── constants.ts         # Preset scenarios, asset/account type configs
    │   ├── tauri.ts             # isTauri() guard + tauriInvoke() wrapper
    │   ├── mockData.ts          # Realistic mock data for browser dev mode
    │   └── currencyContext.tsx  # Base currency React context
    └── components/
        ├── Layout.tsx           # App shell: sidebar + topbar + content area
        ├── Sidebar.tsx          # Icon nav, mini portfolio value
        ├── TopBar.tsx           # Refresh, base currency picker, daily P&L, countdown
        ├── Dashboard.tsx        # Route /
        ├── Holdings.tsx         # Route /holdings
        ├── Performance.tsx      # Route /performance
        ├── StressTest.tsx       # Route /stress
        ├── Rebalance.tsx        # Route /rebalance
        ├── Alerts.tsx           # Route /alerts
        ├── Dividends.tsx        # Route /dividends
        ├── TransactionHistory.tsx  # Route /transactions
        ├── Analytics.tsx        # Route /analytics
        ├── Settings.tsx         # Route /settings
        ├── AddHoldingModal.tsx  # Add/edit holding modal
        ├── ImportHoldingsModal.tsx  # CSV import with preview
        ├── AccountsModal.tsx    # Named account management
        ├── ActionCenter.tsx     # Quick-access alerts + transactions panel
        └── ui/                  # Shared UI primitives (Toast, Badge, Select, …)
```

---

## Data Flow

1. **Price refresh** — The React frontend calls `tauriInvoke('refresh_prices')`. The Rust backend fetches live quotes from Yahoo Finance (User-Agent header required) and FX rates for all currency pairs present in holdings. Results are written to the local SQLite database.
2. **Portfolio snapshot** — `tauriInvoke('get_portfolio')` triggers `build_portfolio_snapshot` in `portfolio.rs`, which reads holdings and cached prices from SQLite, applies FX conversion, and returns a `PortfolioSnapshot` struct serialized as JSON.
3. **Frontend rendering** — The `usePortfolio` hook holds the snapshot in React state. Components read from the hook and format values using `frontend/lib/format.ts` helpers.
4. **Persistence** — The SQLite database lives at `~/Library/Application Support/portfolio-tracker/portfolio.db` (Tauri's `app_data_dir`). All DB access is async via an SQLx connection pool in WAL mode (5 connections).
5. **Browser dev mode** — When running `npm run dev` outside Tauri, `isTauri()` returns `false` and `tauriInvoke()` falls back to mock data from `frontend/lib/mockData.ts`.

---

## Tauri Command Inventory

All commands are invoked via `tauriInvoke()` from `frontend/lib/tauri.ts`.

| Command | Arguments | Returns |
|---------|-----------|---------|
| `get_portfolio` | — | `PortfolioSnapshot` |
| `get_holdings` | — | `Holding[]` |
| `add_holding` | `{ holding }` | `Holding` |
| `update_holding` | `{ holding }` | `Holding` |
| `delete_holding` | `{ id }` | `boolean` |
| `refresh_prices` | — | `RefreshResult` |
| `get_performance` | `{ range }` | `PerformancePoint[]` |
| `run_stress_test_cmd` | `{ scenario }` | `StressResult` |
| `get_accounts` | — | `Account[]` |
| `add_account` | `{ account }` | `Account` |
| `update_account` | `{ id, account }` | `Account` |
| `delete_account` | `{ id }` | `boolean` |
| `search_symbols` | `{ query }` | `SymbolResult[]` |
| `get_transactions` | `{ holdingId? }` | `Transaction[]` |
| `add_transaction` | `{ input }` | `Transaction` |
| `delete_transaction` | `{ id }` | `boolean` |
| `get_realized_gains` | `{ holdingId? }` | `RealizedGainsSummary` |
| `get_dividends` | — | `Dividend[]` |
| `add_dividend` | `{ dividend }` | `Dividend` |
| `delete_dividend` | `{ id }` | `boolean` |
| `get_alerts` | — | `PriceAlert[]` |
| `add_alert` | `{ alert }` | `PriceAlert` |
| `delete_alert` | `{ id }` | `boolean` |
| `get_portfolio_analytics` | — | `PortfolioAnalytics` |
| `get_rebalance_suggestions` | `{ ... }` | `RebalanceSuggestion[]` |
| `import_holdings_csv` | `{ csv }` | `ImportResult` |
| `export_holdings_csv` | — | `string` |
| `backup_database` | — | `ExportPayload` |
| `restore_database` | `{ payload }` | `void` |
| `get_config_cmd` | `{ key }` | `string \| null` |
| `set_config_cmd` | `{ key, value }` | `void` |

All TypeScript types are defined in `frontend/types/portfolio.ts` and mirror their Rust counterparts in `src-tauri/src/types.rs`.
