# Portfolio Tracker

[![CI](https://github.com/GitError/portfolio-tracker/actions/workflows/ci.yml/badge.svg)](https://github.com/GitError/portfolio-tracker/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.94+-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Node](https://img.shields.io/badge/node-22+-green?style=flat-square&logo=node.js)](https://nodejs.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri-24C8D8?style=flat-square&logo=tauri)](https://tauri.app)

A macOS desktop portfolio tracker built with Tauri v2. Tracks stocks, ETFs, crypto, and multi-currency cash positions with live pricing from Yahoo Finance, stress-test simulations, price alerts, dividend tracking, and portfolio rebalancing. All values are displayed in your chosen base currency (default: CAD). Data is stored locally in SQLite — no cloud account required.

---

<!-- Screenshot placeholder -->
<!-- ![Dashboard screenshot](docs/screenshot.png) -->

---

## Download

Latest release: https://github.com/GitError/portfolio-tracker/releases/latest

> macOS installers may show a security warning until signing certificates are configured. See [docs/releases.md](docs/releases.md).

## Getting Started

**Prerequisites:** Rust 1.70+ ([rustup.rs](https://rustup.rs)) · Node.js 22+ ([nodejs.org](https://nodejs.org)) · macOS 11+

```bash
git clone https://github.com/GitError/portfolio-tracker.git
cd portfolio-tracker
npm install
git config core.hooksPath .githooks
```

```bash
cargo tauri dev     # Full Tauri app (Rust + React)
npm run dev         # Frontend only in browser (mock data, no Rust required)
```

**First run:** `cargo tauri dev` is slow the first time — Rust deps compile from source. Subsequent builds are fast.

### Quick setup

1. Launch with `cargo tauri dev`
2. Go to **Holdings** (`⌘2`) → **Add Holding** (`⌘N`) → enter your first position
3. Click **Refresh** (`⌘R`) to fetch live prices
4. Return to the **Dashboard** (`⌘1`) to see live values

---

## Docs

| Document | Description |
|----------|-------------|
| [docs/features.md](docs/features.md) | Full feature guide — every view and how to use it |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Tech stack, directory tree, data flow, Tauri command inventory |
| [docs/roadmap.md](docs/roadmap.md) | Planned features and future work |
| [docs/releases.md](docs/releases.md) | Release process and signing notes |

---

## Contributing

PRs and issues welcome. Open an issue before making large changes.

## License

[MIT](LICENSE)
