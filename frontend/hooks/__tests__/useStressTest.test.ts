import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import type { PortfolioSnapshot, StressScenario } from '../../types/portfolio';
import { MOCK_SNAPSHOT } from '../../lib/mockData';

// Ensure isTauri() returns false
beforeEach(() => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
});

const MILD_CORRECTION: StressScenario = {
  name: 'Mild Correction',
  shocks: { stock: -0.05, etf: -0.05, crypto: -0.1 },
};

const ZERO_SHOCK: StressScenario = {
  name: 'No Shock',
  shocks: {},
};

describe('useStressTest hook (browser/mock path)', () => {
  it('starts with null result, loading=false, error=null', async () => {
    const { useStressTest } = await import('../../hooks/useStressTest');
    const { result } = renderHook(() => useStressTest());

    expect(result.current.result).toBeNull();
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('runTest with null snapshot does nothing', async () => {
    const { useStressTest } = await import('../../hooks/useStressTest');
    const { result } = renderHook(() => useStressTest());

    act(() => {
      void result.current.runTest(MILD_CORRECTION, null);
    });

    await new Promise((r) => setTimeout(r, 50));

    expect(result.current.result).toBeNull();
    expect(result.current.loading).toBe(false);
  });

  it('runTest computes a result locally when not in Tauri', async () => {
    const { useStressTest } = await import('../../hooks/useStressTest');
    const { result } = renderHook(() => useStressTest());

    act(() => {
      void result.current.runTest(MILD_CORRECTION, MOCK_SNAPSHOT as PortfolioSnapshot);
    });

    await waitFor(() => expect(result.current.result).not.toBeNull(), { timeout: 2000 });

    expect(result.current.result!.scenario).toBe('Mild Correction');
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('stressed value is less than current value for a negative shock', async () => {
    const { useStressTest } = await import('../../hooks/useStressTest');
    const { result } = renderHook(() => useStressTest());

    act(() => {
      void result.current.runTest(MILD_CORRECTION, MOCK_SNAPSHOT as PortfolioSnapshot);
    });

    await waitFor(() => expect(result.current.result).not.toBeNull(), { timeout: 2000 });

    const res = result.current.result!;
    expect(res.stressedValue).toBeLessThan(res.currentValue);
    expect(res.totalImpact).toBeLessThan(0);
    expect(res.totalImpactPercent).toBeLessThan(0);
  });

  it('zero-shock scenario leaves currentValue and stressedValue equal', async () => {
    const { useStressTest } = await import('../../hooks/useStressTest');
    const { result } = renderHook(() => useStressTest());

    act(() => {
      void result.current.runTest(ZERO_SHOCK, MOCK_SNAPSHOT as PortfolioSnapshot);
    });

    await waitFor(() => expect(result.current.result).not.toBeNull(), { timeout: 2000 });

    const res = result.current.result!;
    expect(res.stressedValue).toBeCloseTo(res.currentValue, 2);
    expect(res.totalImpact).toBeCloseTo(0, 2);
  });

  it('result includes holdingBreakdown with one entry per holding', async () => {
    const { useStressTest } = await import('../../hooks/useStressTest');
    const { result } = renderHook(() => useStressTest());

    act(() => {
      void result.current.runTest(MILD_CORRECTION, MOCK_SNAPSHOT as PortfolioSnapshot);
    });

    await waitFor(() => expect(result.current.result).not.toBeNull(), { timeout: 2000 });
    expect(result.current.result!.holdingBreakdown.length).toBe(MOCK_SNAPSHOT.holdings.length);
  });

  it('running a second scenario replaces the previous result', async () => {
    const { useStressTest } = await import('../../hooks/useStressTest');
    const { result } = renderHook(() => useStressTest());

    act(() => {
      void result.current.runTest(MILD_CORRECTION, MOCK_SNAPSHOT as PortfolioSnapshot);
    });
    await waitFor(() => expect(result.current.result).not.toBeNull(), { timeout: 2000 });

    const bearMarket: StressScenario = {
      name: 'Bear Market',
      shocks: { stock: -0.2, etf: -0.2, crypto: -0.4 },
    };

    act(() => {
      void result.current.runTest(bearMarket, MOCK_SNAPSHOT as PortfolioSnapshot);
    });
    await waitFor(() => expect(result.current.result!.scenario).toBe('Bear Market'), {
      timeout: 2000,
    });
  });
});
