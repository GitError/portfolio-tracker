# Agent 4: Stress Test Panel

## Role
You are building the Stress Test view for a Tauri v2 portfolio tracker app. This is the most self-contained feature — it takes a portfolio snapshot + user-defined shocks and shows projected impact. Read CLAUDE.md first for types, design tokens, and conventions.

## Context
The stress test engine on the Rust side (Agent 1) takes a `StressScenario` and returns a `StressResult`. For frontend development, you'll mock the calculation in TypeScript so this view works independently in the browser.

## Step 1: Stress Test Hook

Create `src/hooks/useStressTest.ts`:

```typescript
interface UseStressTestReturn {
  result: StressResult | null;
  loading: boolean;
  error: string | null;
  runTest: (scenario: StressScenario) => void;
}
```

- If running in Tauri (`window.__TAURI__`): invoke `run_stress_test` command
- If running in browser (dev): compute stress test locally in TypeScript:
  1. Get portfolio snapshot from usePortfolio hook (or accept as parameter)
  2. For each holding:
     - Find applicable asset-class shock from scenario.shocks (keyed by assetType)
     - Find applicable FX shock (if holding.currency !== 'CAD', look for `fx_{currency}_cad` in shocks)
     - stressedValue = marketValueCad × (1 + assetShock) × (1 + fxShock)
     - impact = stressedValue - marketValueCad
  3. Aggregate totals
  4. Return StressResult

## Step 2: Stress Test View

Create `src/components/StressTest.tsx`:

Two-column layout. Left = scenario controls, Right = results. Both columns in `var(--bg-surface)` panels.

### Left Column: Scenario Configuration

**Preset selector:**
- Dropdown (styled dark select) with presets from constants.ts:
  - Mild Correction, Bear Market, Crypto Winter, CAD Crash, Stagflation, Custom
- Selecting a preset populates all sliders to matching values
- Selecting "Custom" resets all to 0

**Shock sliders section — "Asset Class Shocks":**
- One slider per asset class: Stocks, ETFs, Crypto
- Range: -50% to +50%, step 1%
- Each slider shows: label (left), current value as signed percentage (right), slider (full width below)

**FX shock sliders section — "Currency Shocks":**
- Dynamic: only show sliders for currencies present in portfolio
- Common ones: USD/CAD, EUR/CAD, GBP/CAD
- Same range and style as above
- Label explains direction: "USD/CAD shock" — positive means CAD weakens (USD holdings worth more in CAD)

**Slider styling (critical for Bloomberg feel):**
- Track: thin (4px height), background `var(--border-primary)`
- Filled portion: gradient based on value — red tones for negative, green tones for positive, accent blue at zero
- Thumb: small square (12×12px), `var(--color-accent)`, no border-radius
- Value display: monospace, colored red if negative, green if positive
- Overall: functional, dense, technical — not playful

**"Run Scenario" behavior:**
- Real-time: recalculate as sliders move (debounced 150ms with a simple setTimeout/clearTimeout)
- No explicit "Run" button needed — results update live
- Show a subtle "Calculating..." indicator during debounce

### Right Column: Results

**Summary card (top):**
- Two big numbers side by side:
  - "CURRENT" → total portfolio value (white/neutral)
  - "STRESSED" → stressed total value (colored red if loss, green if gain)
- Arrow between them
- Below: total impact in $ and %, large font, strongly colored
- If impact is negative, subtle red gradient tint on the card background

**Waterfall / Impact Chart:**
- Recharts `BarChart`, horizontal bars (layout="vertical")
- One bar per holding, sorted by impact magnitude (biggest loss first)
- Negative impact bars: `var(--color-loss)`
- Positive impact bars: `var(--color-gain)`
- Y axis: holding symbols (monospace)
- X axis: impact in CAD
- Tooltip: symbol, current value, stressed value, shock applied, impact $ and %

**Breakdown Table:**
- Below the chart, detailed table:
  - Columns: Symbol, Type, Current Value, Shock Applied, Stressed Value, Impact ($), Impact (%)
  - Same terminal table styling as Holdings view: no border-radius, 1px borders, mono numbers, alternating rows
  - Sorted by impact (biggest loss first)
  - Impact column: red/green colored

### Layout

```
┌──────────────────────┬─────────────────────────────┐
│ SCENARIO CONFIG      │ CURRENT → STRESSED          │
│                      │ $200,000   $172,400          │
│ Preset: [Bear Mkt ▼] │ Impact: -$27,600 (-13.8%)   │
│                      │                             │
│ ─── Asset Shocks ─── │ ┌─── Impact by Holding ───┐ │
│ Stocks  [-20%] ═══○  │ │ ████████ NVDA  -$4,200  │ │
│ ETFs    [-20%] ═══○  │ │ ██████  BTC   -$3,800   │ │
│ Crypto  [-40%] ══○   │ │ █████   VOO   -$2,100   │ │
│                      │ │ ...                      │ │
│ ─── FX Shocks ────── │ └──────────────────────────┘ │
│ USD/CAD [-5%]  ═══○  │                             │
│ EUR/CAD [-3%]  ═══○  │ ┌─── Breakdown Table ─────┐ │
│                      │ │ SYM  CUR   STRESS  IMPACT│ │
│                      │ │ ...                      │ │
│                      │ └──────────────────────────┘ │
└──────────────────────┴─────────────────────────────┘
```

Left column: ~35% width. Right column: ~65% width.

### Edge Cases
- No holdings: show EmptyState "Add holdings to run stress tests"
- All shocks at 0: show "NO SCENARIO APPLIED" in results area
- Single asset class portfolio: only relevant sliders are non-zero

## Step 3: Scenario Comparison (Bonus)

If time permits, add a small comparison mode:
- "Compare Scenarios" toggle at the top
- When active: show a bar chart comparing 2-3 preset scenarios side by side
- Grouped bars: total portfolio impact under each scenario
- Helps answer "which scenario hurts most?"

## Verification
- View loads with default "Mild Correction" preset selected
- Moving sliders updates results in real-time
- Switching presets populates sliders correctly
- Waterfall chart and breakdown table show per-holding impact
- All styling matches terminal theme from CLAUDE.md
- Works in browser with mock data (no Tauri required)
