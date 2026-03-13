# Portfolio Tracker — Build Plan & Claude Code Instructions

## Architecture Overview

```
┌─────────────────────────────────────────────┐
│              Tauri v2 Shell                  │
├──────────────────┬──────────────────────────┤
│   Rust Backend   │   React + TS Frontend    │
│                  │                          │
│  • SQLite DB     │  • Dashboard view        │
│  • Yahoo Finance │  • Holdings table        │
│  • FX rates      │  • Performance charts    │
│  • Stress engine │  • Stress test panel     │
│  • Tauri cmds    │  • Add/Edit holdings     │
│                  │                          │
│  rusqlite        │  Recharts + Tailwind     │
│  reqwest         │  @tauri-apps/api         │
│  serde           │                          │
└──────────────────┴──────────────────────────┘
```

## Key Decisions

- **Base currency**: CAD (all values converted and displayed in CAD)
- **Input method**: Manual entry only (v1)
- **Asset classes**: Stocks, ETFs, Crypto, Cash (multi-currency)
- **Style**: Dark terminal / Bloomberg aesthetic
- **Data persistence**: Local SQLite
- **Price source**: Yahoo Finance (free, no API key)
- **Stress tests**: User-defined shock percentages per asset class + FX

---

## Data Model

### `holdings` table
| Column      | Type    | Notes                                    |
|-------------|---------|------------------------------------------|
| id          | TEXT PK | UUID                                     |
| symbol      | TEXT    | Ticker (AAPL, BTC-USD, etc.) or "CASH"   |
| name        | TEXT    | Display name                             |
| asset_type  | TEXT    | stock / etf / crypto / cash              |
| quantity    | REAL    | Number of shares/units/amount            |
| cost_basis  | REAL    | Per-unit cost in original currency       |
| currency    | TEXT    | USD, CAD, EUR, GBP, etc.                 |
| created_at  | TEXT    | ISO 8601                                 |
| updated_at  | TEXT    | ISO 8601                                 |

### `price_cache` table
| Column      | Type    | Notes                                    |
|-------------|---------|------------------------------------------|
| symbol      | TEXT PK | Ticker                                   |
| price       | REAL    | Latest price in native currency          |
| currency    | TEXT    | Native currency of the price             |
| updated_at  | TEXT    | ISO 8601                                 |

### `fx_rates` table
| Column      | Type    | Notes                                    |
|-------------|---------|------------------------------------------|
| pair        | TEXT PK | e.g. "USDCAD", "EURCAD"                 |
| rate        | REAL    | Conversion rate to CAD                   |
| updated_at  | TEXT    | ISO 8601                                 |

---

## Tauri Commands (Rust → JS bridge)

```
get_portfolio()         → Portfolio summary with live valuations in CAD
get_holdings()          → Vec<Holding> raw list
add_holding(h)          → Insert new holding
update_holding(h)       → Update existing holding
delete_holding(id)      → Remove holding
refresh_prices()        → Fetch latest prices + FX, update cache
get_performance(range)  → Historical portfolio value series
run_stress_test(shocks) → Apply shocks, return projected portfolio
```

---

## Stress Test Engine (v1)

Input: a `StressScenario` object:
```json
{
  "name": "Bear Market Lite",
  "shocks": {
    "stock": -0.10,
    "etf": -0.10,
    "crypto": -0.25,
    "fx_usd_cad": -0.05,
    "fx_eur_cad": -0.03
  }
}
```

Logic:
1. Take current portfolio snapshot
2. Apply asset-class shocks to quantities × price
3. Apply FX shocks to non-CAD positions
4. Return: current value, stressed value, delta, per-holding breakdown

---

## Frontend Views

### 1. Dashboard (`/`)
- Total portfolio value (CAD) — big number, green/red delta
- Daily P&L
- Allocation donut chart (by asset type + by currency)
- Top 5 movers (biggest daily % change)
- Last refresh timestamp

### 2. Holdings (`/holdings`)
- Sortable table: Symbol, Name, Type, Qty, Cost Basis, Current Price, Market Value (CAD), Gain/Loss, Weight %
- Inline delete, click-to-edit
- "Add Holding" button → modal form

### 3. Performance (`/performance`)
- Line chart: portfolio value over time
- Range selector: 1D / 1W / 1M / 3M / 1Y / ALL
- Optional S&P 500 (^GSPC) overlay for benchmark comparison

### 4. Stress Test (`/stress`)
- Preset scenarios (dropdown): "Mild Correction", "2008-style", "Crypto Winter", "CAD Crash"
- Custom mode: sliders for each asset class shock (-50% to +50%)
- FX shock sliders
- Results panel: current vs stressed value, waterfall chart showing per-holding impact
- "What if" feel — instant recalculation as sliders move

### 5. Add/Edit Holding (modal)
- Fields: Symbol (with autocomplete later), Name, Type (dropdown), Quantity, Cost Basis, Currency
- Validation: positive numbers, valid currency codes

---

## Visual Design — Dark Terminal / Bloomberg

### Color Palette
- **Background**: `#0a0a0f` (near-black with blue undertone)
- **Surface**: `#12121a` (cards, panels)
- **Border**: `#1e1e2e` (subtle grid lines)
- **Text primary**: `#e0e0e0`
- **Text secondary**: `#6b7280`
- **Green (gain)**: `#00d4aa` (Bloomberg teal-green)
- **Red (loss)**: `#ff4757`
- **Accent**: `#3b82f6` (electric blue for interactive elements)
- **Yellow (warning/neutral)**: `#fbbf24`

### Typography
- **Monospace for numbers**: `JetBrains Mono` or `IBM Plex Mono`
- **UI text**: `IBM Plex Sans` — clean, professional, not generic
- Numbers right-aligned, tabular figures

### Layout
- Sidebar nav (collapsed icons, expand on hover)
- Dense data grid — no wasted space
- Subtle scan-line or noise texture on background (very faint)
- Green/red P&L flash animations on price updates
- Thin 1px borders, no border-radius on data tables (terminal feel)
- Cards with very subtle `box-shadow` or `border` only

### Micro-interactions
- Price update: brief green/red flash on the cell
- Holdings table rows: subtle highlight on hover
- Stress test sliders: real-time recalculation with smooth transitions
- Chart tooltips with crosshair cursor

---

## Claude Code Instructions

Below are step-by-step prompts to feed into Claude Code. Run them sequentially.

---

### Phase 1: Project Scaffold

```
Create a new Tauri v2 project called "portfolio-tracker" with React + TypeScript frontend using Vite. Use Tailwind CSS v4 for styling and Recharts for charts. 

Rust dependencies: tauri 2, serde, serde_json, reqwest (with json feature), tokio (full), chrono (with serde), rusqlite (with bundled feature), uuid (with v4 and serde features).

Frontend dependencies: @tauri-apps/api, @tauri-apps/plugin-shell, recharts, react-router-dom, lucide-react.

Set up the project structure:
- src-tauri/src/main.rs (Tauri setup with DB init)
- src-tauri/src/db.rs (SQLite schema and queries)
- src-tauri/src/commands.rs (all Tauri commands)
- src-tauri/src/price.rs (Yahoo Finance price fetching)
- src-tauri/src/fx.rs (FX rate fetching)  
- src-tauri/src/stress.rs (stress test engine)
- src-tauri/src/types.rs (shared types)
- src/App.tsx (router setup)
- src/components/ (React components)
- src/hooks/ (custom hooks for Tauri command calls)
- src/lib/ (utilities, formatters)
- src/types/ (TypeScript types mirroring Rust)

Base currency is CAD. All portfolio valuations must convert to CAD using live FX rates.
```

### Phase 2: Rust Backend

```
Implement the Rust backend for the portfolio tracker:

1. types.rs: Define Holding, AssetType (stock/etf/crypto/cash), PortfolioSnapshot, StressScenario, StressResult, PriceData, FxRate structs. All with Serialize/Deserialize.

2. db.rs: SQLite setup with three tables:
   - holdings (id TEXT PK, symbol, name, asset_type, quantity REAL, cost_basis REAL, currency TEXT, created_at, updated_at)
   - price_cache (symbol TEXT PK, price REAL, currency TEXT, updated_at)
   - fx_rates (pair TEXT PK, rate REAL, updated_at)
   Provide functions: init_db, insert_holding, update_holding, delete_holding, get_all_holdings, get_cached_prices, upsert_price, upsert_fx_rate.

3. price.rs: Fetch stock/ETF/crypto prices from Yahoo Finance using reqwest. Use the v8 finance API endpoint: https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?interval=1d&range=1d. Parse the JSON to extract regularMarketPrice. Handle errors gracefully. For crypto, symbols are like "BTC-CAD" or "ETH-CAD".

4. fx.rs: Fetch FX rates to CAD. Use Yahoo Finance with symbols like "USDCAD=X", "EURCAD=X", "GBPCAD=X". Provide convert_to_cad(amount, from_currency, rates) helper.

5. stress.rs: StressEngine that takes a portfolio snapshot and a StressScenario { name, shocks: HashMap<String, f64> } where keys are "stock", "etf", "crypto", or FX pairs like "fx_usd_cad". Apply percentage shocks to market values, return StressResult with current_value, stressed_value, delta, per_holding_breakdown.

6. commands.rs: Tauri commands wrapping all the above:
   - get_portfolio() -> PortfolioSnapshot (holdings + prices + FX, all in CAD)
   - get_holdings() -> Vec<Holding>
   - add_holding(holding) -> Holding
   - update_holding(holding) -> Holding  
   - delete_holding(id) -> bool
   - refresh_prices() -> Vec<PriceData> (fetch all prices + FX, update cache)
   - run_stress_test(scenario) -> StressResult
   Use Tauri's managed state with Mutex<Connection> for SQLite.

7. main.rs: Initialize DB on startup, register all commands, set up managed state.
```

### Phase 3: Frontend — Types, Hooks, Layout

```
Build the frontend foundation for the portfolio tracker. Dark terminal/Bloomberg aesthetic.

1. src/types/portfolio.ts: TypeScript types matching Rust structs — Holding, AssetType, PortfolioSnapshot, StressScenario, StressResult, PriceData.

2. src/hooks/usePortfolio.ts: Custom hook using @tauri-apps/api invoke() to call all backend commands. Include loading/error states. Auto-refresh prices on mount.

3. src/lib/format.ts: Formatters — formatCurrency(amount, currency='CAD'), formatPercent(decimal), formatNumber(n), colorForPnl(value) returning green/red class.

4. src/App.tsx: React Router with sidebar layout. Routes: / (Dashboard), /holdings, /performance, /stress.

5. src/components/Layout.tsx: App shell with collapsible sidebar nav. Dark theme:
   - Background: #0a0a0f
   - Surface/cards: #12121a  
   - Borders: #1e1e2e
   - Text: #e0e0e0 / #6b7280
   - Green: #00d4aa, Red: #ff4757, Accent: #3b82f6
   
   Fonts: JetBrains Mono (from Google Fonts) for numbers, IBM Plex Sans for UI text. 
   
   Sidebar: icon-based nav with tooltip labels, active state indicator. Top bar with portfolio total value and last-refresh time.
   
   Terminal aesthetic: sharp corners on data elements, 1px borders, dense layout, monospace numbers, subtle scanline texture on background. No rounded corners on tables.
```

### Phase 4: Dashboard View

```
Build the Dashboard view (src/components/Dashboard.tsx) for the portfolio tracker.

Layout: CSS grid, dense Bloomberg-style panels.

Panels:
1. Hero number: Total portfolio value in CAD (large, JetBrains Mono, with green/red daily P&L underneath)
2. Allocation donut chart (Recharts PieChart): segments by asset type (stock, etf, crypto, cash), dark theme colors, labels outside
3. Currency exposure donut: breakdown by currency (USD, CAD, EUR, etc.)
4. Top movers: small table of top 5 holdings by daily % change, with green/red indicators
5. Quick stats row: total holdings count, best performer, worst performer, cash position

Style: all panels in #12121a surface color, 1px #1e1e2e borders, no border-radius. Green #00d4aa for positive, red #ff4757 for negative. Numbers in JetBrains Mono. Subtle hover states on interactive elements.
```

### Phase 5: Holdings Table

```
Build the Holdings view (src/components/Holdings.tsx).

Full-width sortable data table with columns: Symbol, Name, Type, Qty, Cost Basis, Current Price, Market Value (CAD), Gain/Loss ($), Gain/Loss (%), Weight %.

Features:
- Click column headers to sort (asc/desc toggle)
- Row hover highlight (#1a1a2e)
- Inline delete button (trash icon, confirm dialog)
- Click row to open edit modal
- "Add Holding" button top-right → opens AddHoldingModal

AddHoldingModal (src/components/AddHoldingModal.tsx):
- Fields: Symbol (text input), Name, Type (dropdown: Stock/ETF/Crypto/Cash), Quantity, Cost Basis per unit, Currency (dropdown: CAD/USD/EUR/GBP/JPY/CHF)
- Validation: required fields, positive numbers
- Dark modal with backdrop blur

Table style: no border-radius, 1px borders, monospace numbers right-aligned, alternating row backgrounds (#12121a / #0f0f17). Terminal-density spacing. Type column with colored badges (blue=stock, purple=etf, orange=crypto, green=cash).
```

### Phase 6: Performance Charts

```
Build the Performance view (src/components/Performance.tsx).

Main chart: Recharts AreaChart showing portfolio value over time in CAD.
- Range selector buttons: 1D, 1W, 1M, 3M, 1Y, ALL (styled as terminal-style toggle group)
- Fill gradient from #3b82f6 to transparent
- Crosshair tooltip showing date + value
- Grid lines in #1e1e2e

Below the main chart:
- Daily returns bar chart (small, green/red bars)
- Stats row: total return %, annualized return, max drawdown, Sharpe ratio (if we have enough data)

Note: For v1, historical data will be limited to what we can fetch from Yahoo Finance. We can seed historical portfolio value by storing snapshots in a new SQLite table on each refresh. For now, mock with reasonable sample data and wire up real data incrementally.

Dark theme consistent with rest of app. Chart area background #0a0a0f, no chart border-radius.
```

### Phase 7: Stress Test Panel

```
Build the Stress Test view (src/components/StressTest.tsx).

Two-column layout:

LEFT: Scenario Configuration
- Preset dropdown: "Mild Correction" (stocks -5%, crypto -10%), "Bear Market" (stocks -20%, crypto -40%, fx_usd_cad -5%), "Crypto Winter" (crypto -50%), "CAD Crash" (fx_usd_cad +15%, fx_eur_cad +10%), "Custom"
- Sliders for each shock (range -50% to +50%):
  - Stocks shock %
  - ETFs shock %
  - Crypto shock %
  - Per-currency FX shock: USD/CAD, EUR/CAD, GBP/CAD
- Sliders should be styled: thin track in #1e1e2e, thumb in #3b82f6, negative values shade red, positive shade green
- Real-time recalculation as sliders move (debounced 150ms)

RIGHT: Results
- Big comparison: Current Value → Stressed Value (with delta in $ and %)
- Waterfall chart (Recharts BarChart): showing per-holding impact, sorted by largest loss
- Breakdown table: each holding with current value, stressed value, shock applied, impact

Style: Bloomberg terminal feel. Results panel has a subtle red/green gradient tint based on overall impact. Sharp corners, dense data, monospace numbers.
```

### Phase 8: Polish & Integration

```
Final polish pass on the portfolio tracker:

1. Add a "Refresh" button in the top bar that calls refresh_prices(). Show a subtle loading spinner during fetch. Show last-updated timestamp.

2. Empty states: when no holdings exist, show a centered terminal-style message "NO POSITIONS. ADD HOLDINGS TO BEGIN TRACKING." with a prominent Add button.

3. Error handling: toast notifications for API failures (dark toast, red accent, auto-dismiss).

4. Keyboard shortcuts: Cmd+N to add holding, Cmd+R to refresh prices, 1-4 to switch views.

5. Sidebar: show mini portfolio value at the top, nav items with lucide-react icons (LayoutDashboard, Table, TrendingUp, AlertTriangle).

6. Window chrome: set Tauri window title to "Portfolio Tracker — $XX,XXX.XX CAD" (update dynamically).

7. Add a CLAUDE.md to the project root with project context, architecture notes, and conventions for future development.
```
