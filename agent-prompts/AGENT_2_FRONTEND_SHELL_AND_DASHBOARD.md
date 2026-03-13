# Agent 2: Frontend Shell + Dashboard

## Role
You are building the frontend foundation and dashboard view for a Tauri v2 portfolio tracker app. Read CLAUDE.md first — it defines all types, design tokens, and conventions. The Rust backend is being built in parallel by another agent, so you will mock Tauri invoke calls with realistic data for now.

## Prerequisites
The project should already be scaffolded (by Agent 1). If not, wait for the base project structure. You need the CLAUDE.md, package.json, and Tailwind setup to be in place.

## Step 1: TypeScript Types

Create `src/types/portfolio.ts` — copy the exact types from the CLAUDE.md "Shared TypeScript Types" section. These are the contract. Do not modify them.

## Step 2: Mock Data Layer

Create `src/lib/mockData.ts`:

Generate realistic mock data for development while the backend is being built. Include:
- 8-12 sample holdings: a mix of stocks (AAPL, MSFT, NVDA, TD.TO, RY.TO), ETFs (VOO, QQQ, VFV.TO), crypto (BTC, ETH), and cash positions (USD cash, EUR cash, CAD cash)
- Use realistic prices, quantities, and cost bases
- Generate a mock PortfolioSnapshot with all calculated fields (market values in CAD, gain/loss, weights)
- All CAD conversions should use approximate rates: USD/CAD ~1.36, EUR/CAD ~1.47, GBP/CAD ~1.72
- Total portfolio value should be in the $150k-250k CAD range (realistic personal portfolio)
- Include daily changes: some green, some red, range ±0.5% to ±5%

Create `src/hooks/usePortfolio.ts`:
- Check for `window.__TAURI__` to detect if running in Tauri or browser
- If Tauri: use `invoke()` calls to real backend
- If browser (dev mode): return mock data with simulated 500ms loading delay
- Expose: `{ portfolio, holdings, loading, error, refreshPrices, addHolding, updateHolding, deleteHolding }`

## Step 3: Utility Functions

Create `src/lib/format.ts`:
```typescript
formatCurrency(amount: number, currency?: string): string
// → "$152,340.00 CAD" or "$1,234.56 USD"
// Use Intl.NumberFormat, always 2 decimal places

formatPercent(decimal: number): string
// → "+12.34%" or "-5.67%"
// Always include sign

formatNumber(n: number, decimals?: number): string
// → "1,234.56"

formatCompact(n: number): string
// → "$152.3K" or "$1.2M"
```

Create `src/lib/colors.ts`:
```typescript
pnlColor(value: number): string
// → 'var(--color-gain)' | 'var(--color-loss)' | 'var(--text-secondary)'

pnlClass(value: number): string
// → 'text-gain' | 'text-loss' | 'text-secondary'

assetTypeColor(type: AssetType): string
// → color from design tokens
```

Create `src/lib/constants.ts`:
- Preset stress scenarios (from CLAUDE.md)
- Asset type display config (label, color, icon name)
- Supported currencies list
- Chart range options

## Step 4: App Shell & Layout

Create `src/App.tsx`:
- React Router with BrowserRouter
- Routes: `/` → Dashboard, `/holdings` → Holdings, `/performance` → Performance, `/stress` → StressTest
- Wrap everything in Layout component

Create `src/components/Layout.tsx`:
- Full-height flex layout: sidebar (left) + main content (right)
- Background: `var(--bg-primary)` on body, content area fills remaining space
- Content area has padding, overflow-y auto

Create `src/components/Sidebar.tsx`:
- Fixed-width sidebar: 64px collapsed (icons only), 220px expanded on hover
- Smooth width transition (200ms ease)
- Top section: app logo/name (small "PT" monogram in accent color when collapsed, "Portfolio Tracker" when expanded)
- Nav items with lucide-react icons:
  - LayoutDashboard → "/" (Dashboard)
  - Table2 → "/holdings" (Holdings)
  - TrendingUp → "/performance" (Performance)
  - AlertTriangle → "/stress" (Stress Test)
- Active item: left 2px accent border, bg-surface-hover, text-primary
- Inactive: text-secondary, hover → text-primary
- Bottom section: mini portfolio total value in compact format ("$152.3K"), green/red colored

Create `src/components/TopBar.tsx`:
- Sticky top bar inside content area
- Left: current view title (large, font-sans, font-semibold)
- Right: 
  - Last updated timestamp ("Updated 2m ago" or "Refreshing...")
  - Refresh button (RefreshCw icon, spins during loading)
  - Small daily P&L badge ("+$1,234.56 (+0.82%)")

## Step 5: Dashboard View

Create `src/components/Dashboard.tsx`:

Dense Bloomberg-style grid layout (CSS Grid). All panels in `var(--bg-surface)`, 1px `var(--border-primary)` border, no border-radius.

**Panel 1 — Portfolio Value (spans 2 columns)**
- Total value: massive number, JetBrains Mono, 48px, `var(--text-primary)`
- Below it: daily P&L in dollars and percent, colored green/red
- Below that: total cost basis and total gain/loss in smaller text
- Subtle horizontal rule separator

**Panel 2 — Allocation by Asset Type**
- Recharts PieChart (donut variant, innerRadius=60, outerRadius=90)
- Segments colored by asset type tokens (stock=blue, etf=purple, crypto=orange, cash=green)
- Center label: "Allocation"
- Legend below chart: type name + percentage + dollar value
- Dark theme: no background fill on chart area

**Panel 3 — Currency Exposure**
- Same PieChart style but segmented by currency (CAD, USD, EUR, etc.)
- Use distinct colors: CAD=#00d4aa, USD=#3b82f6, EUR=#8b5cf6, GBP=#f59e0b
- Center label: "Currency"

**Panel 4 — Top Movers (spans 2 columns)**
- Small table: top 5 holdings by absolute daily % change
- Columns: Symbol, Name, Change %, Change $
- Sorted by magnitude (biggest movers first)
- Green/red coloring on change values
- Terminal-density: small text, tight row padding

**Panel 5 — Quick Stats Row (spans full width)**
- Horizontal stat cards:
  - Total positions count
  - Best performer (symbol + %)
  - Worst performer (symbol + %)
  - Cash position total (CAD)
  - Portfolio beta (placeholder "—" for v1)

**Grid layout suggestion:**
```
[value  ] [value  ] [alloc  ]
[movers ] [movers ] [currency]
[stats  ] [stats  ] [stats  ]
```

Use `grid-template-columns: 1fr 1fr 1fr` with responsive adjustments. Gap: 1px (terminal grid feel) or 8px.

## Step 6: Shared UI Components

Create `src/components/ui/Badge.tsx`:
- Tiny badge for asset types
- Props: `type: AssetType`
- Style: uppercase, text-[10px], font-semibold, px-2 py-0.5, 2px border-radius
- Color mapped by asset type

Create `src/components/ui/Spinner.tsx`:
- Simple rotating border spinner
- Props: `size?: 'sm' | 'md' | 'lg'`
- Color: `var(--color-accent)`

Create `src/components/ui/EmptyState.tsx`:
- Centered text: monospace, `var(--text-secondary)`
- Props: `message: string`, `action?: { label: string, onClick: () => void }`
- Terminal aesthetic: "> NO POSITIONS FOUND" style message

Create `src/components/ui/Toast.tsx`:
- Fixed bottom-right positioning
- Dark background, colored left border (red=error, green=success, blue=info)
- Auto-dismiss after 4 seconds
- Provide a ToastProvider context + useToast hook

## Styling Notes
- Import JetBrains Mono and IBM Plex Sans from Google Fonts in index.html
- Set `body { background: var(--bg-primary); color: var(--text-primary); font-family: var(--font-sans); }`
- Add subtle scanline texture: a CSS `::after` pseudo-element on the body with a repeating-linear-gradient (1px transparent / 1px rgba(0,0,0,0.03)) — very subtle, almost invisible
- All number elements get `font-family: var(--font-mono)`
- Scrollbar styling: thin, dark track, subtle thumb

## Verification
Run `npm run dev` — the app should load in browser with mock data, showing the sidebar, top bar, and a fully populated dashboard with charts and data. All panels should be styled in the dark terminal theme.
