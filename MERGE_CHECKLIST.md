# Merge Checklist — Combining Agent Outputs

## Overview
After all 4 agents complete their work, use this checklist to merge everything into a working app. The merge order matters.

---

## Prerequisites
- [ ] Agent 1 has a compiling Rust backend (`cargo build` succeeds)
- [ ] Agent 2 has a running frontend shell (`npm run dev` shows sidebar + dashboard)
- [ ] Agent 3 has Holdings and Performance views working with mock data
- [ ] Agent 4 has Stress Test view working with mock data

---

## Merge Order

### 1. Start from Agent 1's repo (it has the full scaffold)
```bash
cd portfolio-tracker
```

### 2. Merge Agent 2's frontend work
Copy these files/dirs from Agent 2 into the Agent 1 project:
- [ ] `src/index.css` (design tokens, global styles)
- [ ] `src/App.tsx` (router setup)
- [ ] `src/types/portfolio.ts`
- [ ] `src/hooks/usePortfolio.ts`
- [ ] `src/lib/format.ts`
- [ ] `src/lib/colors.ts`
- [ ] `src/lib/constants.ts`
- [ ] `src/lib/mockData.ts`
- [ ] `src/components/Layout.tsx`
- [ ] `src/components/Sidebar.tsx`
- [ ] `src/components/TopBar.tsx`
- [ ] `src/components/Dashboard.tsx`
- [ ] `src/components/ui/*` (Badge, Spinner, EmptyState, Toast)
- [ ] `index.html` (check for Google Fonts link)
- [ ] `tailwind.config.ts`

### 3. Merge Agent 3's views
- [ ] `src/components/Holdings.tsx`
- [ ] `src/components/AddHoldingModal.tsx`
- [ ] `src/components/Performance.tsx`

Check for conflicts:
- Agent 3 might have created its own mock data or modified `usePortfolio.ts` — use Agent 2's version as the base, merge any additions
- Ensure imports resolve (Agent 3's components should import from the same paths)

### 4. Merge Agent 4's stress test
- [ ] `src/hooks/useStressTest.ts`
- [ ] `src/components/StressTest.tsx`

Check:
- Ensure it imports StressScenario presets from `src/lib/constants.ts` (Agent 2 should have defined these)
- If Agent 4 defined its own constants, merge them into the single constants file

---

## Post-Merge Integration

### Connect frontend hooks to real Tauri backend
- [ ] In `usePortfolio.ts`: verify the `invoke()` call signatures match `commands.rs` exactly
  - `invoke('get_portfolio')` → no args
  - `invoke('add_holding', { holding })` → HoldingInput (no id/timestamps)
  - `invoke('update_holding', { holding })` → full Holding
  - `invoke('delete_holding', { id })` → string id
  - `invoke('refresh_prices')` → no args
  - `invoke('get_performance', { range })` → string range like "1M"
- [ ] In `useStressTest.ts`: verify `invoke('run_stress_test', { scenario })` matches
- [ ] Check that Rust types (camelCase via serde) match TypeScript types exactly

### Verify type alignment
Run this mental check on every field:
```
Rust struct field (snake_case) → serde camelCase → TypeScript interface field
─────────────────────────────────────────────────────────────────────────────
asset_type: AssetType    → assetType    → assetType: AssetType ✓
cost_basis: f64          → costBasis    → costBasis: number    ✓
gain_loss_percent: f64   → gainLossPercent → gainLossPercent: number ✓
```

### Test the integration
- [ ] `cargo tauri dev` — app launches with real Tauri window
- [ ] Add a test holding via the modal → check it persists after app restart
- [ ] Refresh prices → verify Yahoo Finance calls succeed (check Rust logs)
- [ ] Check FX conversion: USD holding should show CAD-converted values
- [ ] Run a stress test → verify results match manual calculation
- [ ] Navigate all 4 views — no blank screens, no console errors

---

## Common Issues & Fixes

### Yahoo Finance 403
**Symptom**: Price fetch fails, all prices show 0
**Fix**: Ensure reqwest client has User-Agent header set. Check `main.rs` creates the client with `.user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")`

### Type mismatch: "expected X, got undefined"  
**Symptom**: Frontend crashes when receiving real data from Tauri
**Fix**: Check Rust struct field names — serde rename_all camelCase must be on EVERY struct. Common miss: nested structs or enum variants.

### SQLite "database is locked"
**Symptom**: Concurrent operations fail
**Fix**: Ensure all DB access goes through the single `Mutex<Connection>` in Tauri state. Don't hold the lock across await points.

### Tauri invoke not found
**Symptom**: `invoke('command_name')` throws "unknown command"
**Fix**: Check `main.rs` generate_handler! includes ALL commands. Every #[tauri::command] function must be listed.

### Charts don't render
**Symptom**: Recharts shows blank area
**Fix**: Ensure ResponsiveContainer has a parent with explicit height. Recharts needs a sized container.

### Fonts not loading
**Symptom**: Falls back to system fonts
**Fix**: Check index.html has the Google Fonts `<link>` tag, and Tauri CSP config allows fonts.googleapis.com and fonts.gstatic.com

---

## Final Polish (post-merge)

- [ ] Add keyboard shortcuts (Cmd+N, Cmd+R, 1-4 for views)
- [ ] Update Tauri window title dynamically with portfolio value
- [ ] Add error toast notifications for failed API calls
- [ ] Test window resize — all layouts should be responsive
- [ ] Build production: `cargo tauri build` → creates .dmg
- [ ] Test the .dmg on a clean install
