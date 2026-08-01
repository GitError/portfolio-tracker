import { fxShockKey } from './constants';
import type { PortfolioSnapshot, StressResult, StressScenario } from '../types/portfolio';

/**
 * Standalone TypeScript reimplementation of the Rust stress-test calculation in
 * `portfolio-core/src/stress.rs` (the canonical `run_stress_test`, shared by
 * `src-tauri` and `portfolio-mcp`). This file does NOT call into the Rust code —
 * it's a hand-maintained port, used only when no Tauri backend is available
 * (browser dev mode and scenario-comparison previews) since those need a result
 * synchronously without a round trip to `run_stress_test_cmd`. The desktop app
 * always defers to the Rust implementation for its primary stress-test results.
 *
 * This is a deliberate parallel implementation, not incidental duplication: keep it
 * in sync with `run_stress_test` in `portfolio-core/src/stress.rs` if the shock
 * model changes. See #645 / #646 (frontend/lib/__tests__/scenarioMath.test.ts has
 * a cross-language parity test that documents matching output for a known input).
 */
export function computeStressImpact(
  snapshot: PortfolioSnapshot,
  scenario: StressScenario
): StressResult {
  let totalStressed = 0;

  const holdingBreakdown = snapshot.holdings.map((h) => {
    // Asset-level shock: cash is excluded — only FX shocks may affect cash positions.
    const assetShock = h.assetType === 'cash' ? 0 : (scenario.shocks[h.assetType] ?? 0);

    const fxShock =
      h.currency.toUpperCase() === snapshot.baseCurrency.toUpperCase()
        ? 0
        : (scenario.shocks[fxShockKey(h.currency, snapshot.baseCurrency)] ?? 0);

    const currentValue = h.marketValueCad;
    const stressedValue = currentValue * (1 + assetShock) * (1 + fxShock);
    const impact = stressedValue - currentValue;
    const shockApplied = (1 + assetShock) * (1 + fxShock) - 1;

    totalStressed += stressedValue;

    return {
      holdingId: h.id,
      symbol: h.symbol,
      name: h.name,
      currentValue,
      stressedValue,
      impact,
      shockApplied,
    };
  });

  const currentValue = snapshot.totalValue;
  const totalImpact = totalStressed - currentValue;
  const totalImpactPercent = currentValue !== 0 ? (totalImpact / currentValue) * 100 : 0;

  return {
    scenario: scenario.name,
    currentValue,
    stressedValue: totalStressed,
    totalImpact,
    totalImpactPercent,
    holdingBreakdown,
  };
}
