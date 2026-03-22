# Portfolio Tracker — Project Contract

## Overview
A macOS desktop portfolio tracker built with Tauri v2 (Rust backend + React/TypeScript frontend). Tracks stocks, ETFs, crypto, and multi-currency cash positions with live pricing, stress-test simulations, price alerts, dividend tracking, and portfolio rebalancing. Values displayed in the user's chosen base currency (default: CAD).

## Architecture

```
portfolio-tracker/
├── CLAUDE.md                    ← you are here
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── migrations/              ← SQLx migrations (0001–0006)
│   └── src/
│       ├── main.rs              ← Tauri entry point
│       ├── lib.rs               ← App bootstrap, state init, command registration
│       ├── config.rs            ← App-level constants (DB name, user-agent, TTLs)
│       ├── types.rs             ← Shared Rust types (Serialize/Deserialize, camelCase)
│       ├── db.rs                ← SQLite schema, migrations, CRUD (async SQLx)
│       ├── commands.rs          ← #[tauri::command] thin wrappers
│       ├── portfolio.rs         ← build_portfolio_snapshot + helpers
│       ├── csv.rs               ← CSV import/export helpers
│       ├── price.rs             ← Yahoo Finance price fetching
│       ├── fx.rs                ← FX rate fetching + conversion helpers
│       ├── search.rs            ← Symbol search via Yahoo Finance
│       ├── analytics.rs         ← Realized gains + portfolio analytics
│       └── stress.rs            ← Stress test engine
├── frontend/
│   ├── App.tsx                  ← Router, providers, keyboard shortcut wiring
│   ├── main.tsx                 ← React entry
│   ├── index.css                ← Tailwind + global styles + design tokens
│   ├── types/
│   │   └── portfolio.ts         ← TypeScript types (mirrors Rust types exactly)
│   ├── hooks/
│   │   ├── usePortfolio.ts      ← Tauri invoke wrapper + shared portfolio state
│   │   ├── useStressTest.ts     ← Stress test invocation + state
│   │   ├── useConfig.ts         ← Persistent key/value config via Tauri
│   │   ├── useAutoRefresh.ts    ← Interval-based auto price refresh + countdown
│   │   └── useKeyboardShortcuts.ts ← Global keyboard shortcut handler
│   ├── lib/
│   │   ├── format.ts            ← Currency/number/percent formatters
│   │   ├── colors.ts            ← PnL color helpers
│   │   ├── constants.ts         ← Preset scenarios, asset/account type configs
│   │   └── currencyContext.tsx  ← Base currency React context
│   └── components/
│       ├── Layout.tsx            ← App shell: sidebar + topbar + content area
│       ├── Sidebar.tsx           ← Icon nav, mini portfolio value
│       ├── TopBar.tsx            ← Refresh, base currency picker, daily P&L, countdown
│       ├── Dashboard.tsx         ← Dashboard view (route: /)
│       ├── Holdings.tsx          ← Holdings table view (route: /holdings)
│       ├── Performance.tsx       ← Performance charts (route: /performance)
│       ├── StressTest.tsx        ← Stress test panel (route: /stress)
│       ├── Rebalance.tsx         ← Rebalancing view (route: /rebalance)
│       ├── Alerts.tsx            ← Price alerts view (route: /alerts)
│       ├── Dividends.tsx         ← Dividend tracking view (route: /dividends)
│       ├── Settings.tsx          ← Settings panel (route: /settings)
│       ├── AddHoldingModal.tsx   ← Add/edit holding modal
│       ├── ImportHoldingsModal.tsx ← CSV import with preview
│       └── ui/
│           ├── Toast.tsx         ← Notification toast
│           ├── Badge.tsx         ← Asset type badge
│           ├── Spinner.tsx       ← Loading spinner
│           ├── EmptyState.tsx    ← No-data placeholder
│           ├── Select.tsx        ← Custom select component
│           ├── SymbolSearch.tsx  ← Symbol search autocomplete
│           └── KeyboardShortcutsOverlay.tsx ← `?` help overlay
├── public/
├── index.html
├── package.json
├── tsconfig.json
├── tailwind.config.ts
└── vite.config.ts
```

## Tech Stack

| Layer     | Technology                        |
|-----------|-----------------------------------|
| Shell     | Tauri v2                          |
| Backend   | Rust (tokio, reqwest, sqlx, tracing) |
| Frontend  | React 18 + TypeScript + Vite      |
| Styling   | Tailwind CSS v4                   |
| Charts    | Recharts                          |
| Icons     | lucide-react                      |
| Router    | react-router-dom v7               |

## Conventions

### Rust
- All types in `types.rs`, re-exported from other modules as needed
- Commands in `commands.rs` are thin wrappers: validate input → call domain fn → return result
- Domain logic lives in `portfolio.rs` (snapshot), `csv.rs` (import/export), `analytics.rs`, `stress.rs`
- Use `Result<T, String>` for command return types (Tauri convention)
- Database accessed via async `SqlitePool` (SQLx + WAL mode, 5 connections); migrations in `src-tauri/migrations/`
- All DB operations go through `db.rs` — no raw SQL anywhere else
- Use `serde(rename_all = "camelCase")` on all structs exposed to frontend
- reqwest calls must include header `User-Agent: Mozilla/5.0` (Yahoo Finance blocks bare requests)
- Use `tracing::error!/warn!/info!` for logging — never `eprintln!`

### TypeScript
- Types in `frontend/types/portfolio.ts` must mirror Rust types exactly (camelCase)
- **Always use `tauriInvoke()` from `frontend/lib/tauri.ts`** — never raw `invoke()` from `@tauri-apps/api/core`; this includes an `isTauri()` guard for browser dev mode
- Hooks in `frontend/hooks/` wrap `tauriInvoke()` calls with loading/error states
- All currency values are `number` (f64 from Rust), formatted at render time only
- Use `frontend/lib/format.ts` for ALL number display — never inline `toFixed()` etc.
- `SUPPORTED_CURRENCIES` is defined and exported from `frontend/lib/constants.ts` — import from there

### Styling
- Tailwind utility classes only — no CSS modules, no styled-components
- Design tokens defined as CSS variables in `index.css` (see Design System below)
- Monospace font for ALL numbers: `font-mono` class
- No border-radius on data tables and data cells
- Minimal border-radius (2px max) on buttons and badges only

---

## Shared TypeScript Types

All agents MUST use these exact types. The authoritative source is `frontend/types/portfolio.ts`.

Key types (abbreviated — read the source file for full definitions):

```typescript
export type AssetType = 'stock' | 'etf' | 'crypto' | 'cash';
export type AccountType = 'tfsa' | 'rrsp' | 'fhsa' | 'taxable' | 'crypto' | 'cash' | 'other';

export interface Holding {
  id: string; symbol: string; name: string; assetType: AssetType;
  account: AccountType; quantity: number; costBasis: number;
  currency: string; exchange: string; targetWeight: number;
  createdAt: string; updatedAt: string;
  indicatedAnnualDividend: number | null;
  indicatedAnnualDividendCurrency: string | null;
  dividendFrequency: 'monthly' | 'quarterly' | 'semi-annual' | 'annual' | 'irregular' | null;
  maturityDate: string | null;
}

export interface HoldingWithPrice extends Holding {
  currentPrice: number; currentPriceCad: number; marketValueCad: number;
  costValueCad: number; gainLoss: number; gainLossPercent: number;
  weight: number; targetValue: number; targetDeltaValue: number;
  targetDeltaPercent: number; dailyChangePercent: number;
}

export interface PortfolioSnapshot {
  holdings: HoldingWithPrice[];
  totalValue: number; totalCost: number; totalGainLoss: number;
  totalGainLossPercent: number; dailyPnl: number;
  lastUpdated: string; baseCurrency: string;
  totalTargetWeight: number; targetCashDelta: number;
  realizedGains: number; annualDividendIncome: number;
}

export interface RefreshResult {
  prices: PriceData[];
  failedSymbols: string[];
  triggeredAlerts: string[];
  alertErrors?: string[];
}

export interface PriceAlert {
  id: string; symbol: string; direction: 'above' | 'below';
  threshold: number; currency: string; note: string;
  triggered: boolean; createdAt: string;
}

export interface PreviewRow {
  // ... other fields ...
  status: 'ready' | 'cash' | 'duplicate' | 'invalid_symbol' | 'validation_failed';
}
```

// ── Tauri Command Signatures ── (use tauriInvoke, not invoke)

```
tauriInvoke('get_portfolio')                          → PortfolioSnapshot
tauriInvoke('get_holdings')                           → Holding[]
tauriInvoke('add_holding', { holding })               → Holding
tauriInvoke('update_holding', { holding })            → Holding
tauriInvoke('delete_holding', { id })                 → boolean
tauriInvoke('refresh_prices')                         → RefreshResult
tauriInvoke('get_performance', { range })             → PerformancePoint[]
tauriInvoke('run_stress_test_cmd', { scenario })      → StressResult
tauriInvoke('get_accounts')                           → Account[]
tauriInvoke('add_account', { account })               → Account
tauriInvoke('update_account', { id, account })        → Account
tauriInvoke('delete_account', { id })                 → boolean
tauriInvoke('search_symbols', { query })              → SymbolResult[]
tauriInvoke('get_transactions', { holdingId? })       → Transaction[]
tauriInvoke('add_transaction', { input })             → Transaction
tauriInvoke('delete_transaction', { id })             → boolean
tauriInvoke('get_realized_gains', { holdingId? })     → RealizedGainsSummary
tauriInvoke('get_dividends')                          → Dividend[]
tauriInvoke('add_dividend', { dividend })             → Dividend
tauriInvoke('delete_dividend', { id })                → boolean
tauriInvoke('get_alerts')                             → PriceAlert[]
tauriInvoke('add_alert', { alert })                   → PriceAlert
tauriInvoke('delete_alert', { id })                   → boolean
tauriInvoke('get_portfolio_analytics')                → PortfolioAnalytics
tauriInvoke('get_rebalance_suggestions', { ... })     → RebalanceSuggestion[]
tauriInvoke('import_holdings_csv', { csv })           → ImportResult
tauriInvoke('export_holdings_csv')                    → string
tauriInvoke('backup_database')                        → ExportPayload
tauriInvoke('restore_database', { payload })          → void
tauriInvoke('get_config_cmd', { key })                → string | null
tauriInvoke('set_config_cmd', { key, value })         → void
```

---

## Design System

### CSS Variables (define in `frontend/index.css`)

```css
:root {
  /* Backgrounds */
  --bg-primary: #0a0a0f;
  --bg-surface: #12121a;
  --bg-surface-hover: #1a1a2e;
  --bg-surface-alt: #0f0f17;

  /* Borders */
  --border-primary: #1e1e2e;
  --border-subtle: #16161f;

  /* Text */
  --text-primary: #e0e0e0;
  --text-secondary: #6b7280;
  --text-muted: #4b5563;

  /* Semantic */
  --color-gain: #00d4aa;
  --color-loss: #ff4757;
  --color-accent: #3b82f6;
  --color-warning: #fbbf24;

  /* Asset type colors */
  --color-stock: #3b82f6;
  --color-etf: #8b5cf6;
  --color-crypto: #f59e0b;
  --color-cash: #00d4aa;

  /* Typography */
  --font-mono: 'JetBrains Mono', 'SF Mono', monospace;
  --font-sans: 'IBM Plex Sans', -apple-system, sans-serif;
}
```

### Tailwind Extensions (in `tailwind.config.ts`)

Extend theme with the above colors. Add `fontFamily: { mono: [var(--font-mono)], sans: [var(--font-sans)] }`.

### Component Patterns

- **Data tables**: No border-radius. 1px `var(--border-primary)` borders. Alternating rows `var(--bg-surface)` / `var(--bg-surface-alt)`. Hover: `var(--bg-surface-hover)`.
- **Cards/Panels**: Background `var(--bg-surface)`, 1px border, no or minimal (2px) border-radius.
- **Numbers**: Always `font-mono`, right-aligned in tables, use `format.ts` helpers.
- **PnL coloring**: Positive → `var(--color-gain)`, negative → `var(--color-loss)`, zero → `var(--text-secondary)`.
- **Badges**: Tiny, uppercase, `text-[10px]`, 2px border-radius, colored by asset type.
- **Buttons**: Primary = `var(--color-accent)` bg, secondary = transparent with border. 2px border-radius max.
- **Modals**: Centered, backdrop blur, `var(--bg-surface)` background, max-width 480px.

### Fonts
Import in `index.html`:
```html
<link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:wght@400;500;600&family=JetBrains+Mono:wght@400;500;600&display=swap" rel="stylesheet">
```

---

## Preset Stress Scenarios

```typescript
export const PRESET_SCENARIOS: StressScenario[] = [
  {
    name: 'Mild Correction',
    shocks: { stock: -0.05, etf: -0.05, crypto: -0.10 }
  },
  {
    name: 'Bear Market',
    shocks: { stock: -0.20, etf: -0.20, crypto: -0.40, fx_usd_cad: -0.05 }
  },
  {
    name: 'Crypto Winter',
    shocks: { crypto: -0.50 }
  },
  {
    name: 'CAD Crash',
    shocks: { fx_usd_cad: 0.15, fx_eur_cad: 0.10, fx_gbp_cad: 0.10 }
  },
  {
    name: 'Stagflation',
    shocks: { stock: -0.15, etf: -0.12, crypto: -0.20, fx_usd_cad: 0.08 }
  }
];
```

---

## Notes for Agents
- **Always use `tauriInvoke()` from `frontend/lib/tauri.ts`** for Tauri commands, never raw `invoke()` from `@tauri-apps/api/core`. The wrapper includes an `isTauri()` guard so browser dev mode works.
- Check `isTauri()` to detect browser vs Tauri context — use realistic mock data from `frontend/lib/mockData.ts` in browser mode.
- All components must handle: loading state, error state, and empty state.
- Yahoo Finance requests MUST include a User-Agent header or they will 403.
- The SQLite DB file lives in Tauri's app data directory (`app_data_dir`), NOT in the project folder.
- All timestamps are ISO 8601 UTC strings.
- Logging: use `tracing::error!/warn!/info!` macros in Rust — never `eprintln!`.
- DB access: use `SqlitePool` (async SQLx), not `rusqlite`. All DB functions are `async fn` in `db.rs`.

## Testing
- Rust: `cargo test` (unit tests in each module)
- Frontend: Vitest + React Testing Library
- E2E: Playwright
- Coverage target: 80%+