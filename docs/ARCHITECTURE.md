# Architecture

Technical reference for the Portfolio Tracker codebase.

---

## Tech Stack

| Layer     | Technology                                        |
|-----------|---------------------------------------------------|
| Shell     | [Tauri v2](https://tauri.app)                     |
| Backend   | Rust — tokio, reqwest, sqlx, chrono, uuid         |
| Frontend  | React 19 + TypeScript + Vite                      |
| Styling   | Tailwind CSS v4                                   |
| Charts    | [Recharts](https://recharts.org)                  |
| Icons     | [lucide-react](https://lucide.dev)                |
| Router    | react-router-dom v7                               |
| Database  | SQLite via SQLx (WAL mode, async connection pool) |

### Current Dependency Baseline

As of the 2026-07-28 housekeeping pass, the app targets Node.js 22+ for frontend tooling and uses React 19.2, Vite 8.0, Vitest 4.1.9, TypeScript 5.9, ESLint 10.5, Tailwind 4.2, lucide-react 1.27, recharts 3.10, i18next 26.3 / react-i18next 17.0, react-router-dom 7.14, and @tauri-apps/api 2.11 on the frontend; Tauri 2.10, Tokio 1.50, chrono 0.4.44, and uuid 1.23 on the backend. **SQLx is split across the workspace:** `src-tauri` requires `^0.9` (Cargo.lock resolves to 0.9.0), but `portfolio-mcp` still pins `^0.8` (resolves to 0.8.6) — the two binaries run different SQLx major versions. Worth aligning `portfolio-mcp/Cargo.toml` to `^0.9` in a follow-up if there's no reason for the split.

---

## Directory Tree

```
portfolio-tracker/                          # Cargo workspace root
├── Cargo.toml                              # Workspace members: src-tauri, portfolio-mcp
├── src-tauri/                              # Rust backend for Tauri desktop app
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── migrations/                         # SQLx migrations (0001–0011)
│   └── src/
│       ├── main.rs                         # Tauri entry point
│       ├── lib.rs                          # App bootstrap, state init, command registration
│       ├── config.rs                       # App-level constants (DB name, user-agent, TTLs)
│       ├── error.rs                        # AppError — shared Tauri command error type
│       ├── types.rs                        # All shared Rust types (serde camelCase)
│       ├── commands/                       # Tauri command handlers (thin wrappers over domain logic)
│       │   ├── mod.rs                      # Command registration, DbState + shared state structs
│       │   ├── accounts.rs                 # Account CRUD commands
│       │   ├── alerts.rs                   # Price alert commands
│       │   ├── analytics.rs                # Analytics/rebalance commands
│       │   ├── backup.rs                   # backup_database / restore_database
│       │   ├── config.rs                   # get_config_cmd / set_config_cmd (key allowlist)
│       │   ├── dividends.rs                # Dividend commands
│       │   ├── import.rs                   # CSV import/export commands
│       │   ├── portfolio.rs                # Holdings CRUD + get_portfolio
│       │   ├── prices.rs                   # refresh_prices, search_symbols, get_performance
│       │   ├── stress.rs                   # run_stress_test_cmd
│       │   └── transactions.rs             # Transaction commands
│       ├── commands_tests.rs               # Command-level validation tests
│       ├── db.rs                           # SQLite schema, migrations, CRUD (async SQLx)
│       ├── portfolio.rs                    # build_portfolio_snapshot + helpers
│       ├── csv.rs                          # CSV import/export helpers
│       ├── price.rs                        # Yahoo Finance price fetching
│       ├── fx.rs                           # FX rate fetching + conversion helpers
│       ├── search.rs                       # Symbol search via Yahoo Finance
│       ├── analytics.rs                    # Realized gains + portfolio analytics
│       └── stress.rs                       # Stress test engine
├── portfolio-mcp/                          # Standalone MCP server binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                         # MCP server entry (stdio transport)
│       ├── db.rs                           # SQLite access via SQLx
│       ├── snapshot.rs                     # Portfolio snapshot calculation
│       ├── stress.rs                       # Stress test execution
│       ├── types.rs                        # MCP-specific types
│       └── tools/                          # 14 MCP tools
│           ├── holdings.rs                 # list_holdings, add_holding, delete_holding
│           ├── transactions.rs             # list_transactions, add_transaction, delete_transaction
│           ├── alerts.rs                   # list_alerts, add_alert, delete_alert, reset_alert
│           ├── portfolio.rs                # get_portfolio_snapshot
│           ├── stress.rs                   # run_stress_test
│           └── config.rs                   # get_config, set_config
└── frontend/                               # React frontend
    ├── App.tsx                             # Router, providers, keyboard shortcut wiring
    ├── main.tsx                            # React entry point
    ├── index.css                           # Tailwind + global styles + design tokens
    ├── types/
    │   └── portfolio.ts                    # TypeScript types (mirrors Rust structs exactly)
    ├── hooks/
    │   ├── usePortfolio.ts                 # Tauri invoke wrapper + shared portfolio state
    │   ├── useStressTest.ts                # Stress test invocation + state
    │   ├── useConfig.ts                    # Persistent key/value config via Tauri
    │   ├── useAutoRefresh.ts               # Interval-based auto price refresh + countdown
    │   ├── useKeyboardShortcuts.ts         # Global keyboard shortcut handler
    │   ├── useTheme.ts                     # Dark mode theme context
    │   ├── useLanguage.ts                  # i18next internationalization hook
    │   └── useActionInsights.ts            # Portfolio action recommendations
    ├── lib/
    │   ├── format.ts                       # Currency/number/percent formatters
    │   ├── colors.ts                       # PnL color helpers
    │   ├── constants.ts                    # Preset scenarios, asset/account type configs
    │   ├── tauri.ts                        # isTauri() guard + tauriInvoke() wrapper
    │   ├── mockData.ts                     # Realistic mock data for browser dev mode
    │   └── currencyContext.tsx             # Base currency React context
    └── components/
        ├── Layout.tsx                      # App shell: sidebar + topbar + content area
        ├── Sidebar.tsx                     # Icon nav, mini portfolio value
        ├── TopBar.tsx                      # Refresh, base currency picker, daily P&L, countdown
        ├── Dashboard.tsx                   # Route /
        ├── Holdings.tsx                    # Route /holdings [paginated]
        ├── Performance.tsx                 # Route /performance
        ├── StressTest.tsx                  # Route /stress (composes ShockSliders, PresetScenarioSelector, StressResultsTable, ScenarioComparison, StressTestInfo, ResilienceSummary)
        ├── Rebalance.tsx                   # Route /rebalance
        ├── Alerts.tsx                      # Route /alerts
        ├── TransactionHistory.tsx          # Route /transactions [paginated]
        ├── Analytics.tsx                   # Route /analytics — sector/country breakdown, risk metrics, realized gains
        ├── Dividends.tsx                   # Route /dividends
        ├── Settings.tsx                    # Route /settings
        ├── Help.tsx                        # Route /help — keyboard shortcuts reference
        ├── AddHoldingModal.tsx             # Add/edit holding modal
        ├── AddTransactionModal.tsx         # Log a buy/sell transaction
        ├── ImportHoldingsModal.tsx         # CSV import with preview
        ├── CostBasisModal.tsx              # Cost-basis selection modal (AVCO, FIFO)
        ├── AccountsModal.tsx               # Named account management (create/delete)
        ├── ActionCenter.tsx                # Side panel: recent alert triggers + fast transaction entry
        ├── KeyboardShortcutsOverlay.tsx    # `?`-triggered shortcuts overlay
        └── ui/                             # Shared UI primitives (Toast, Badge, Select, …)
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

All commands are invoked via `tauriInvoke()` from `frontend/lib/tauri.ts`. Command handlers live in `src-tauri/src/commands/` (split by domain, registered in `commands/mod.rs`) and return `Result<T, AppError>` — see the `AppError` note in Conventions below.

> **Pagination:** every `*_paginated` command takes 1-indexed `{ page, pageSize }` (page ≥ 1, pageSize 1–500) — not `{ offset, limit }` — and returns `PaginatedResult<T>`.

### Portfolio & Snapshot

| Command | Arguments | Returns |
|---------|-----------|---------|
| `get_portfolio` | — | `PortfolioSnapshot` |
| `get_portfolio_analytics` | — | `PortfolioAnalytics` |

### Holdings

| Command | Arguments | Returns |
|---------|-----------|---------|
| `get_holdings` | — | `Holding[]` *deprecated: use get_holdings_paginated* |
| `get_holdings_paginated` | `{ page, pageSize }` | `PaginatedResult<Holding>` |
| `add_holding` | `{ holding }` | `Holding` |
| `update_holding` | `{ holding }` | `Holding` |
| `delete_holding` | `{ id }` | `boolean` |

### Transactions

| Command | Arguments | Returns |
|---------|-----------|---------|
| `get_transactions` | `{ holdingId? }` | `Transaction[]` *deprecated: use get_transactions_paginated* |
| `get_transactions_paginated` | `{ holdingId?, page, pageSize }` | `PaginatedResult<Transaction>` |
| `add_transaction` | `{ input }` | `Transaction` |
| `delete_transaction` | `{ id }` | `boolean` |

### Alerts

| Command | Arguments | Returns |
|---------|-----------|---------|
| `get_alerts` | — | `PriceAlert[]` *deprecated: use get_alerts_paginated* |
| `get_alerts_paginated` | `{ page, pageSize }` | `PaginatedResult<PriceAlert>` |
| `add_alert` | `{ alert }` | `PriceAlert` (validates threshold is finite and positive) |
| `delete_alert` | `{ id }` | `boolean` |
| `reset_alert` | `{ id }` | `PriceAlert` |

### Price & Market Data

| Command | Arguments | Returns |
|---------|-----------|---------|
| `refresh_prices` | — | `RefreshResult` |
| `search_symbols` | `{ query }` | `SymbolResult[]` |
| `get_symbol_price` | `{ symbol }` | `PriceData` |
| `get_cached_prices` | — | `PriceData[]` |
| `get_performance` | `{ range }` | `PerformancePoint[]` |

### Accounts

| Command | Arguments | Returns |
|---------|-----------|---------|
| `get_accounts` | — | `Account[]` |
| `add_account` | `{ account }` | `Account` |
| `update_account` | `{ id, account }` | `Account` |
| `delete_account` | `{ id }` | `boolean` |

### Dividends

| Command | Arguments | Returns |
|---------|-----------|---------|
| `get_dividends` | — | `Dividend[]` |
| `get_dividends_paginated` | `{ page, pageSize }` | `PaginatedResult<Dividend>` |
| `add_dividend` | `{ dividend }` | `Dividend` |
| `delete_dividend` | `{ id }` | `boolean` |

### Analytics

| Command | Arguments | Returns |
|---------|-----------|---------|
| `get_realized_gains` | `{ holdingId? }` | `RealizedGainsSummary` |
| `get_rebalance_suggestions` | `{ ... }` | `RebalanceSuggestion[]` |

### Stress Testing

| Command | Arguments | Returns |
|---------|-----------|---------|
| `run_stress_test_cmd` | `{ scenario }` | `StressResult` |

### Import / Export

| Command | Arguments | Returns |
|---------|-----------|---------|
| `import_holdings_csv` | `{ csvContent }` | `ImportResult` |
| `preview_import_csv` | `{ csvContent }` | `PreviewImportResult` |
| `export_holdings_csv` | — | `string` |

### Backup / Restore

| Command | Arguments | Returns |
|---------|-----------|---------|
| `backup_database` | `{ destinationPath }` | `string` — path the backup was written to; a bare filename resolves under `app_data_dir`, WAL is checkpointed first |
| `restore_database` | `{ sourcePath }` | `string` — status message; source is verified via SQLite magic bytes + `PRAGMA integrity_check` before it replaces the live DB |

### Configuration

| Command | Arguments | Returns |
|---------|-----------|---------|
| `get_config_cmd` | `{ key }` | `string \| null` |
| `set_config_cmd` | `{ key, value }` | `void` — `key` must be on the allowlist in `commands/config.rs` |

All TypeScript types are defined in `frontend/types/portfolio.ts` and mirror their Rust counterparts in `src-tauri/src/types.rs`.

---

## MCP Server (portfolio-mcp)

**portfolio-mcp** is a standalone Rust binary that exposes the Portfolio Tracker database over the Model Context Protocol (MCP) using stdio transport. This allows AI assistants and agents to read and write portfolio data, enabling AI-powered analysis and recommendations.

### Architecture

- **Transport**: stdio-based MCP (Model Context Protocol)
- **Database**: Connects to the same SQLite database as the Tauri app (read+write)
- **Tools**: 14 MCP tools exposing holdings, transactions, alerts, portfolio snapshots, stress tests, and configuration
- **Prices**: Reads cached prices populated by the Tauri app; does not fetch live prices itself

### Tools Exposed

**Holdings & Accounts**
- `list_holdings` — List all current holdings
- `add_holding` — Create a new holding
- `delete_holding` — Soft-delete a holding by UUID

**Transactions**
- `list_transactions` — List buy/sell transactions (optionally filtered by holdingId)
- `add_transaction` — Record a new transaction
- `delete_transaction` — Soft-delete a transaction

**Price Alerts**
- `list_alerts` — List all price alerts
- `add_alert` — Create a price alert
- `delete_alert` — Delete an alert
- `reset_alert` — Reset a triggered alert

**Portfolio Analysis**
- `get_portfolio_snapshot` — Full snapshot with live prices, market values, gains/losses, weights
- `run_stress_test` — Apply asset-class and FX shocks to portfolio

**Configuration**
- `get_config` — Read a config value (e.g., base_currency)
- `set_config` — Write a config value

### Build & Deployment

```bash
. ~/.cargo/env
cargo build -p portfolio-mcp --release
```

Binary location: `target/release/portfolio-mcp`

### Setup in Claude Code

Add to `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "portfolio": {
      "command": "/absolute/path/to/target/release/portfolio-mcp",
      "env": {
        "PORTFOLIO_DB_PATH": "/Users/YOU/Library/Application Support/com.portfolio-tracker.app/portfolio.db"
      }
    }
  }
}
```

### Configuration

| Environment Variable | Default | Description |
|----------------------|---------|-------------|
| `PORTFOLIO_DB_PATH` | `~/Library/Application Support/com.portfolio-tracker.app/portfolio.db` | Path to the SQLite database |
| `RUST_LOG` | `portfolio_mcp=info` | Log level filter; logs go to stderr, never stdout |

### Implementation Notes

- The MCP server **reads from the Tauri app's database**; it does not initialize a new database.
- Prices and FX rates are cached by the Tauri app. The MCP server does not fetch live data independently.
- `realized_gains` and `annual_dividend_income` in `get_portfolio_snapshot` report as `0` in the MCP context; use the Tauri app for authoritative figures.
- All tools validate input and return structured JSON responses following MCP conventions.
- For full documentation, see `portfolio-mcp/README.md`.
