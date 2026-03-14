# Portfolio Tracker

[![CI](https://github.com/GitError/portfolio-tracker/actions/workflows/ci.yml/badge.svg)](https://github.com/GitError/portfolio-tracker/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.94+-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Node](https://img.shields.io/badge/node-22+-green?style=flat-square&logo=node.js)](https://nodejs.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

A macOS desktop portfolio tracker built with Tauri v2. Tracks stocks, ETFs, crypto, and multi-currency cash positions with live pricing from Yahoo Finance and real-time stress-test simulations. All values are displayed in CAD.

The app runs as a native macOS window with a Bloomberg-inspired dark terminal UI. Holdings are stored locally in SQLite — no cloud account required.

---

## Screenshot

> _Dashboard showing portfolio value, allocation breakdown, and top movers._

![Dashboard](docs/screenshot-dashboard.png)

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

## Features

- **Dashboard** — portfolio value, daily P&L, asset allocation donut, currency exposure, top movers
- **Holdings** — sortable table with add/edit/delete; supports stocks, ETFs, crypto, and multi-currency cash
- **Performance** — 2-year area chart with range selector (1D–ALL), daily returns bar chart, drawdown/volatility stats
- **Stress Test** — live slider-driven scenario simulation; preset scenarios (Bear Market, Crypto Winter, CAD Crash, Stagflation); waterfall chart with per-holding impact breakdown
- **Multi-currency** — USD, EUR, GBP, and more; all converted to CAD at live rates

---

## Getting Started

### Prerequisites

- **Rust** 1.70+ — [rustup.rs](https://rustup.rs)
- **Node.js** 22+ — [nodejs.org](https://nodejs.org)
- macOS 11+

### Install

```bash
git clone https://github.com/GitError/portfolio-tracker.git
cd portfolio-tracker
npm install
git config core.hooksPath .githooks
```

### Run in development

```bash
npm run tauri dev
```

Starts both the Vite dev server and the Tauri window. The frontend falls back to mock data in browser-only mode (`npm run dev`).

### First-time setup

1. Launch with `npm run tauri dev`
2. Navigate to **Holdings** (sidebar or press `2`)
3. Click **Add Holding** and enter your first position
4. Return to the **Dashboard** to see live values

---

## Development

```bash
# Frontend
npm run dev           # Vite dev server (browser, mock data)
npm run lint          # ESLint
npm run typecheck     # TypeScript check
npm run format        # Prettier (write)
npm run format:check  # Prettier (check only)
npm run test          # Vitest (run once)
npm run test:watch    # Vitest (watch)
npm run test:coverage # Coverage report (v8)

# Rust
cd src-tauri
cargo test            # Unit tests (db, stress engine, fx)
cargo clippy          # Linter
cargo fmt             # Formatter
```

### Git hooks

Located in `.githooks/`, activated during install above.

| Hook | Runs |
|------|------|
| pre-commit | lint, typecheck, format check, cargo fmt |
| pre-push | full test suite (frontend + Rust) |
| post-merge | reinstalls deps if lock files changed |

---

## Contributing

PRs and issues are welcome. Open an issue before making large changes.

---

## License

[MIT](LICENSE)
