import { useState, useCallback } from 'react';
import type { PortfolioSnapshot, StressResult, StressScenario } from '../types/portfolio';

const isTauri = (): boolean => typeof window !== 'undefined' && '__TAURI__' in window;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

function computeLocally(snapshot: PortfolioSnapshot, scenario: StressScenario): StressResult {
  let totalStressed = 0;

  const holdingBreakdown = snapshot.holdings.map((h) => {
    const assetShock = scenario.shocks[h.assetType] ?? 0;
    const fxKey = `fx_${h.currency.toLowerCase()}_cad`;
    const fxShock = h.currency.toUpperCase() === 'CAD' ? 0 : (scenario.shocks[fxKey] ?? 0);

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
  const totalImpactPercent =
    currentValue !== 0 ? (totalImpact / currentValue) * 100 : 0;

  return {
    scenario: scenario.name,
    currentValue,
    stressedValue: totalStressed,
    totalImpact,
    totalImpactPercent,
    holdingBreakdown,
  };
}

export interface UseStressTestReturn {
  result: StressResult | null;
  loading: boolean;
  error: string | null;
  runTest: (scenario: StressScenario, snapshot: PortfolioSnapshot | null) => void;
}

export function useStressTest(): UseStressTestReturn {
  const [result, setResult] = useState<StressResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const runTest = useCallback(
    async (scenario: StressScenario, snapshot: PortfolioSnapshot | null) => {
      if (!snapshot) return;
      setLoading(true);
      setError(null);
      try {
        if (isTauri()) {
          const res = await tauriInvoke<StressResult>('run_stress_test_cmd', { scenario });
          setResult(res);
        } else {
          // Small async tick so loading state renders
          await new Promise((r) => setTimeout(r, 0));
          setResult(computeLocally(snapshot, scenario));
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setLoading(false);
      }
    },
    []
  );

  return { result, loading, error, runTest };
}
