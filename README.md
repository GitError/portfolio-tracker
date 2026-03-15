# Portfolio Tracker

[![CI](https://github.com/GitError/portfolio-tracker/actions/workflows/ci.yml/badge.svg)](https://github.com/GitError/portfolio-tracker/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.94+-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Node](https://img.shields.io/badge/node-22+-green?style=flat-square&logo=node.js)](https://nodejs.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri-24C8D8?style=flat-square&logo=tauri)](https://tauri.app)

A macOS desktop portfolio tracker built with Tauri v2. Tracks stocks, ETFs, crypto, and multi-currency cash positions with live pricing from Yahoo Finance and real-time stress-test simulations. All values are displayed in CAD.

The app runs as a native macOS window with a Bloomberg-inspired dark terminal UI. Holdings are stored locally in SQLite — no cloud account required.

**Docs:** [Feature Guide](docs/features.md) · [Roadmap](docs/roadmap.md)

---

## Screenshot

![Portfolio Tracker dashboard showing portfolio value, allocation, and holdings](docs/screenshot-dashboard.png)

---

## Features

- **Dashboard** — Real-time portfolio value and daily P&L, asset allocation donut by type and currency, top movers sorted by daily change
- **Holdings** — Add, edit, and delete positions across stocks, ETFs, crypto, and multi-currency cash; sortable table with live price and gain/loss columns
- **Performance** — Historical portfolio value area chart with configurable time ranges (1D–ALL), daily returns bar chart, drawdown and volatility stats
- **Stress Testing** — Apply preset or custom shock scenarios (Bear Market, Crypto Winter, CAD Crash, Stagflation) to see projected impact with per-holding waterfall breakdown
- **Multi-currency** — USD, EUR, GBP, CHF, JPY, and more; all positions converted to CAD at live FX rates fetched on demand

---

## Tech Stack

| Layer     | Technology                                        |
|-----------|---------------------------------------------------|
| Shell     | [Tauri v2](https://tauri.app)                     |
| Backend   | Rust — tokio, reqwest, rusqlite                   |
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
│       ├── main.rs     # Tauri bootstrap + state
│       ├── commands.rs # Tauri command handlers
│       ├── db.rs       # SQLite schema + CRUD
│       ├── price.rs    # Yahoo Finance price fetching
│       ├── fx.rs       # FX rate fetching + conversion
│       └── stress.rs   # Stress test engine
└── src/                # React frontend
    ├── components/     # Views: Dashboard, Holdings, Performance, StressTest
    ├── hooks/          # usePortfolio, useStressTest
    ├── lib/            # Formatters, colours, constants
    └── types/          # TypeScript types (mirrors Rust structs)
```

The Rust backend exposes Tauri commands that the React frontend calls via `invoke()`. The SQLite database lives in the macOS app data directory. Price data is fetched from Yahoo Finance on demand and cached locally.

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
2. Navigate to **Holdings** (sidebar or press `2`)
3. Click **Add Holding** and enter your first position
4. Return to the **Dashboard** to see live values

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
cargo test                # Unit tests (db, stress engine, fx)
cargo clippy              # Linter
cargo fmt                 # Formatter
```

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
