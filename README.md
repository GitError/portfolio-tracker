# Portfolio Tracker

[![CI](https://github.com/GitError/portfolio-tracker/actions/workflows/ci.yml/badge.svg)](https://github.com/GitError/portfolio-tracker/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.94+-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Node](https://img.shields.io/badge/node-22+-green?style=flat-square&logo=node.js)](https://nodejs.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri-24C8D8?style=flat-square&logo=tauri)](https://tauri.app)

A macOS desktop portfolio tracker built with Tauri v2. Tracks stocks, ETFs, crypto, and multi-currency cash positions with live pricing from Yahoo Finance, real-time stress-test simulations, price alerts, and dividend tracking. All values are displayed in your chosen base currency (default: CAD).

The app runs as a native macOS window with a Bloomberg-inspired dark terminal UI. Holdings and all data are stored locally in SQLite — no cloud account required.

**Docs:** [Feature Guide](docs/features.md) · [Roadmap](docs/roadmap.md) · [Release Guide](docs/releases.md)

---

---

## Download

- Latest release: https://github.com/GitError/portfolio-tracker/releases/latest
- All releases: https://github.com/GitError/portfolio-tracker/releases

Release builds are published from tags matching `v*.*.*` and first appear as draft GitHub Releases for review.

> **Signing note:** macOS and Windows installers may show platform security warnings until signing certificates are configured. See [docs/releases.md](docs/releases.md) for the signing secrets and release flow.

---

## Features

- **Dashboard** — Real-time portfolio value and daily P&L, asset allocation donut by type and currency, top movers sorted by daily change
- **Holdings** — Add, edit, and delete positions across stocks, ETFs, crypto, and multi-currency cash; account tagging (TFSA, RRSP, Taxable); sortable table with live price and gain/loss columns
- **CSV Import / Export** — Bulk-import holdings from a CSV file with symbol validation and import preview; export your full portfolio back to CSV at any time
- **Performance** — Historical portfolio value area chart with configurable time ranges (1W–ALL), daily returns bar chart, drawdown and volatility stats; data recorded on every price refresh
- **Stress Testing** — Apply 11 preset or fully custom shock scenarios (Bear Market, Crypto Winter, CAD Crash, Stagflation, and more) to see projected impact with per-holding waterfall breakdown
- **Rebalancing** — Set target allocation weights per holding; see drift from target, required trades, and deployable cash guidance
- **Price Alerts** — Set above/below price threshold alerts per symbol; triggered alerts are flagged in the UI
- **Dividend Tracking** — Record dividend payments per unit with ex-date and pay-date; view payout history and totals by symbol
- **Settings** — Configure base currency (CAD, USD, EUR, GBP, AUD, CHF, JPY), auto-refresh interval (1m–1hr), and cost basis method (AVCO / FIFO)
- **Multi-currency** — USD, EUR, GBP, CHF, JPY, and more; all positions converted to base currency at live FX rates fetched on demand
- **Keyboard shortcuts** — Full keyboard navigation; press `?` to see all shortcuts
- **Backup / Restore** — Export and import all holdings as JSON for portable backups

---

## Tech Stack

| Layer     | Technology                                        |
|-----------|---------------------------------------------------|
| Shell     | [Tauri v2](https://tauri.app)                     |
| Backend   | Rust — tokio, reqwest, rusqlite, chrono, uuid     |
| Frontend  | React 18 + TypeScript + Vite                      |
| Styling   | Tailwind CSS v4                                   |
| Charts    | [Recharts](https://recharts.org)                  |
| Icons     | [lucide-react](https://lucide.dev)                |
| Router    | react-router-dom v7                               |
| Database  | SQLite (bundled via rusqlite)                     |

---

## Architecture

```
portfolio-tracker/
├── src-tauri/          # Rust backend
│   └── src/
│       ├── main.rs     # Tauri entry point
│       ├── lib.rs      # App bootstrap, state init, command registration
│       ├── config.rs   # App-level constants (DB name, user-agent, etc.)
│       ├── types.rs    # All shared Rust types (serde camelCase)
│       ├── commands.rs # Tauri command handlers (thin wrappers over domain logic)
│       ├── db.rs       # SQLite schema, migrations, CRUD
│       ├── price.rs    # Yahoo Finance price fetching
│       ├── fx.rs       # FX rate fetching + conversion helpers
│       ├── search.rs   # Symbol search via Yahoo Finance
│       └── stress.rs   # Stress test engine
└── src/                # React frontend
    ├── App.tsx         # Router, providers, keyboard shortcut wiring
    ├── components/
    │   ├── Dashboard.tsx           # Route /
    │   ├── Holdings.tsx            # Route /holdings
    │   ├── Performance.tsx         # Route /performance
    │   ├── StressTest.tsx          # Route /stress
    │   ├── Rebalance.tsx           # Route /rebalance
    │   ├── Alerts.tsx              # Route /alerts
    │   ├── Dividends.tsx           # Route /dividends
    │   ├── Settings.tsx            # Route /settings
    │   ├── AddHoldingModal.tsx     # Add / edit holding dialog
    │   ├── ImportHoldingsModal.tsx # CSV import dialog
    │   ├── Layout.tsx              # App shell: sidebar + topbar + content
    │   ├── Sidebar.tsx             # Icon nav, mini portfolio value
    │   ├── TopBar.tsx              # Refresh, base currency picker, daily P&L
    │   └── ui/                     # Shared UI primitives (Toast, Badge, Select, …)
    ├── hooks/
    │   ├── usePortfolio.ts         # Tauri invoke wrapper + portfolio state
    │   ├── useConfig.ts            # Persistent key/value config (SQLite via Tauri)
    │   ├── useAutoRefresh.ts       # Interval-based auto price refresh
    │   ├── useStressTest.ts        # Stress test invocation + state
    │   └── useKeyboardShortcuts.ts # Global keyboard shortcut handler
    ├── lib/
    │   ├── format.ts               # Currency / number / percent formatters
    │   ├── colors.ts               # PnL color helpers
    │   ├── constants.ts            # Preset scenarios, asset/account configs
    │   └── currencyContext.tsx     # Base currency React context
    └── types/
        └── portfolio.ts            # TypeScript types (mirrors Rust structs exactly)
```

The Rust backend exposes Tauri commands that the React frontend calls via `invoke()`. The SQLite database lives in the macOS app data directory (`~/Library/Application Support/portfolio-tracker/portfolio.db`). Price data is fetched from Yahoo Finance on demand and cached locally between refreshes.

---

## Getting Started

### Prerequisites

- **Rust** 1.70+ — [rustup.rs](https://rustup.rs)
- **Node.js** 22+ — [nodejs.org](https://nodejs.org)
- macOS 11+

> **Platform note:** This project targets macOS (Tauri uses WKWebView on mac) and is developed and tested there. Tauri technically supports Windows and Linux, but those platforms are untested. The frontend can be run standalone via `npm run dev` on any OS — useful for UI development without Tauri.

### Install

```bash
git clone https://github.com/GitError/portfolio-tracker.git
cd portfolio-tracker
npm install
git config core.hooksPath .githooks
```

### Run

```bash
cargo tauri dev     # Full Tauri app (Rust backend + React frontend)
npm run dev         # Frontend only in browser (mock data, no Rust required)
```

> **First run:** `cargo tauri dev` will be slow on the first build — rusqlite compiles SQLite from source. Subsequent builds are fast.

### First-time setup

1. Launch with `cargo tauri dev`
2. Navigate to **Holdings** (`2` or `⌘2`)
3. Click **Add Holding** (`⌘N`) and enter your first position
4. Click **Refresh** (`⌘R`) to fetch live prices
5. Return to the **Dashboard** (`1`) to see live values

---

## Development

```bash
# Full Tauri app
cargo tauri dev           # Start Tauri app with hot-reload frontend

# Frontend only (browser, mock data)
npm run dev               # Vite dev server

# Code quality
npm run lint              # ESLint
npm run typecheck         # TypeScript check
npm run format            # Prettier (write)
npm run format:check      # Prettier (check only)
npm run review            # lint + typecheck + format check in one pass

# Tests
npm run test              # Vitest (run once)
npm run test:watch        # Vitest (watch)
npm run test:coverage     # Coverage report (v8)

# Rust
cd src-tauri
cargo test                # Unit tests (db, stress engine, fx, commands)
cargo clippy              # Linter
cargo fmt                 # Formatter
```

### Releases

```bash
./scripts/bump-version.sh 0.2.0
git commit -am "chore: bump version to 0.2.0"
git tag v0.2.0
git push && git push --tags
```

Pushing a `v*.*.*` tag triggers `.github/workflows/release.yml`, which builds cross-platform Tauri installers and creates a draft GitHub Release with attached artifacts.

### Git hooks

Located in `.githooks/`, activated by `git config core.hooksPath .githooks` during install.

| Hook | Runs |
|------|------|
| pre-commit | ESLint, TypeScript check, Prettier check, `cargo fmt --check` (fast, ~5s) |
| pre-push | Staged: lint → build (`npm run build` + `cargo build`) → tests → clippy → Claude review on main/dev |
| post-merge | Reinstalls deps if `package-lock.json` or `Cargo.lock` changed |

The pre-push hook runs in stages so failures surface early:

1. **Lint + format** (~5s) — ESLint, Prettier, `cargo fmt`
2. **Build** (~30–60s warm) — `npm run build` (tsc strict + Vite) and `cargo build` (full compilation)
3. **Tests** (~15–30s) — Vitest and `cargo test`
4. **Clippy** (~10–20s) — `cargo clippy -D warnings`
5. **Claude review** (main/dev only) — AI review of the diff with a proceed/abort prompt

**Escape hatch:** if you need to push urgently (e.g. a docs fix) and don't want to wait, use:

```bash
git push --no-verify
```

CI will still catch any issues — `--no-verify` only skips local hooks.

---

## Contributing

PRs and issues are welcome. Open an issue before making large changes.

---

## License

[MIT](LICENSE)
