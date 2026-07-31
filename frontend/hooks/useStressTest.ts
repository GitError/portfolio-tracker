import { useState, useCallback } from 'react';
import type { PortfolioSnapshot, StressResult, StressScenario } from '../types/portfolio';
import { isTauri, tauriInvoke } from '../lib/tauri';
import { computeStressImpact } from '../lib/scenarioMath';

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
          setResult(computeStressImpact(snapshot, scenario));
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
