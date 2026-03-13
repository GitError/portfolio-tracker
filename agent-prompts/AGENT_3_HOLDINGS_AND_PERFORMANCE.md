# Agent 3: Holdings Table + Performance Charts

## Role
You are building the Holdings and Performance views for a Tauri v2 portfolio tracker app. Read CLAUDE.md first — it defines all types, design tokens, and conventions. Use the mock data layer from `src/hooks/usePortfolio.ts` (built by Agent 2). If it's not ready yet, create your own local mocks matching the types in CLAUDE.md.

## Step 1: Holdings Table View

Create `src/components/Holdings.tsx`:

Full-width sortable data table. This is the core data view — it needs to feel like a Bloomberg terminal grid.

**Table columns:**
| Column | Align | Font | Content |
|--------|-------|------|---------|
| Symbol | left | mono, bold | Ticker symbol |
| Name | left | sans | Display name, truncated |
| Type | left | sans | AssetType badge component |
| Qty | right | mono | Quantity, 2-4 decimal places |
| Cost Basis | right | mono | Per-unit cost in original currency |
| Price | right | mono | Current price in original currency |
| Mkt Value (CAD) | right | mono | Market value converted to CAD |
| Gain/Loss | right | mono | Dollar gain/loss, green/red colored |
| G/L % | right | mono | Percent gain/loss, green/red colored |
| Weight | right | mono | Portfolio weight percentage |
| Actions | center | — | Edit (Pencil icon) + Delete (Trash2 icon) |

**Sorting:**
- Click column header to sort ascending, click again for descending
- Show sort indicator: ▲ or ▼ next to active column header
- Default sort: Weight descending (largest positions first)
- Store sort state in local component state

**Row behavior:**
- Alternating row backgrounds: `var(--bg-surface)` / `var(--bg-surface-alt)`
- Hover: `var(--bg-surface-hover)` with transition
- No border-radius on any table element
- 1px `var(--border-primary)` borders: bottom on each row, right on each cell (subtle grid)
- Dense padding: py-2 px-3

**Header row:**
- Sticky top
- Background: `var(--bg-surface-alt)`
- Text: `var(--text-secondary)`, uppercase, text-xs, font-semibold, tracking-wider
- 1px bottom border

**Footer row:**
- Sticky bottom
- Show totals: total market value, total gain/loss, total gain/loss %
- Bold, slightly larger text
- Separated by 2px top border

**Delete flow:**
- Click trash icon → inline confirmation: row flashes red, "Delete?" text replaces the action buttons, with Confirm/Cancel
- On confirm: call deleteHolding, remove from list with fade-out animation

**Top bar (above table):**
- Left: "Holdings" title + count badge ("12 positions")
- Right: "Add Holding" button (Plus icon + text, accent color)
- Optional: filter/search input for symbol lookup

**Empty state:**
- If no holdings: show EmptyState component
- Message: "> NO POSITIONS. ADD HOLDINGS TO BEGIN TRACKING."
- Centered Add Holding button below

## Step 2: Add/Edit Holding Modal

Create `src/components/AddHoldingModal.tsx`:

Dark modal overlay with backdrop blur.

**Props:**
```typescript
interface Props {
  isOpen: boolean;
  onClose: () => void;
  onSave: (holding: HoldingInput) => void;
  editingHolding?: Holding; // if present, we're editing
}
```

**Form fields:**
- **Symbol**: text input, uppercase transform, placeholder "AAPL"
- **Name**: text input, placeholder "Apple Inc."
- **Type**: select dropdown — Stock, ETF, Crypto, Cash
  - When "Cash" is selected: hide Symbol and Price fields, show only Currency and Amount
- **Quantity**: number input, step 0.01, min 0
- **Cost Basis**: number input (per unit), step 0.01, min 0, label "Cost Per Unit"
- **Currency**: select dropdown — CAD, USD, EUR, GBP, JPY, CHF, AUD

**Validation:**
- All fields required
- Quantity and Cost Basis must be > 0
- Show inline error messages below invalid fields
- Disable Save button until form is valid

**Layout:**
- Modal: `var(--bg-surface)` background, 1px border, max-width 480px, centered
- Backdrop: black 60% opacity with backdrop-filter blur(4px)
- Title: "Add Holding" or "Edit Holding" based on mode
- Two-column grid for fields where it makes sense (Type + Currency on same row)
- Buttons at bottom: Cancel (secondary) + Save (primary accent)

**Style notes:**
- Input fields: `var(--bg-primary)` background, 1px `var(--border-primary)` border, `var(--text-primary)` text
- Focus state: `var(--color-accent)` border
- Select dropdowns: same dark styling, custom arrow
- No border-radius on inputs (terminal feel), or max 2px

## Step 3: Performance View

Create `src/components/Performance.tsx`:

Time-series portfolio performance chart with range selection.

**Mock data for v1:**
Generate realistic mock performance data in the component (or in a helper):
- Generate daily portfolio values for the past 2 years
- Start at ~$180K CAD, general uptrend with drawdowns
- Add realistic daily variance (±0.5% typical, occasional ±2-3% moves)
- Include a mock "crash" period (e.g., -15% over 2 weeks) and recovery

**Main chart — Portfolio Value:**
- Recharts `AreaChart`, responsive container
- X axis: dates (formatted based on range: "Mar 13" for short, "Mar 2026" for long)
- Y axis: CAD value, formatted compact ("$150K", "$200K")
- Area fill: gradient from `var(--color-accent)` opacity 0.3 at top to transparent at bottom
- Line stroke: `var(--color-accent)`, 2px
- Grid lines: `var(--border-subtle)`, dashed
- No chart border-radius, background transparent (inherits panel bg)

**Range selector:**
- Horizontal button group above chart: 1D, 1W, 1M, 3M, 6M, 1Y, ALL
- Style: monospace text, uppercase, tight spacing
- Active: accent background, no border-radius (or 2px max)
- Inactive: transparent, border, text-secondary
- Clicking a range filters the data accordingly

**Crosshair tooltip:**
- Recharts custom Tooltip
- Shows: Date, Portfolio Value (CAD), Daily Change ($), Daily Change (%)
- Dark background, 1px border, monospace numbers

**Below the main chart:**

**Daily Returns Bar Chart:**
- Recharts `BarChart`, height ~120px
- Each bar = one day's return percentage
- Positive bars: `var(--color-gain)`, negative: `var(--color-loss)`
- X axis synced with main chart range
- Y axis: percent

**Stats Row:**
- Horizontal cards below charts showing:
  - Total Return: $ and %
  - Period High / Period Low
  - Max Drawdown (% from peak)
  - Volatility (annualized std dev of daily returns)
  - Best Day / Worst Day
- All calculated from the displayed range data
- Stats in monospace, colored appropriately

**Chart interaction:**
- Range change: smooth data transition
- Hover: crosshair cursor spanning both charts (if feasible with Recharts, otherwise just tooltip)

## Verification
- Holdings page: table renders with mock data, sorting works on all columns, add modal opens and validates
- Performance page: chart renders with mock data, range selection filters data, stats update accordingly
- Both views use the dark terminal theme consistently
- All numbers use format.ts helpers, all colors follow design tokens
