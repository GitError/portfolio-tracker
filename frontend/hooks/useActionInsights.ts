import { useMemo } from 'react';
import type {
  ActionInsight,
  HoldingWithPrice,
  InsightDirection,
  InsightSeverity,
  PortfolioSnapshot,
} from '../types/portfolio';

// ─── Severity sort order ───────────────────────────────────────────────────
const SEVERITY_RANK: Record<InsightSeverity, number> = {
  critical: 0,
  warning: 1,
  info: 2,
};

// ─── Pure builder function (testable without React) ───────────────────────

export function buildInsights(
  snapshot: PortfolioSnapshot,
  holdings: HoldingWithPrice[]
): ActionInsight[] {
  if (holdings.length === 0 || snapshot.totalValue <= 0) return [];

  const insights: ActionInsight[] = [];

  // ── 1. target_drift ───────────────────────────────────────────────────────
  for (const holding of holdings) {
    if (!holding.targetWeight || holding.targetWeight <= 0) continue;

    // Both holding.weight and holding.targetWeight are in 0–100 percent scale from Rust.
    const drift = Math.abs(holding.weight - holding.targetWeight);
    if (drift <= 5) continue;

    const severity: InsightSeverity = drift > 10 ? 'critical' : 'warning';
    const driftPct = drift.toFixed(1);
    const actualPct = holding.weight.toFixed(1);
    const targetPct = holding.targetWeight.toFixed(1);
    const direction = holding.weight > holding.targetWeight ? 'overweight' : 'underweight';
    const insightDirection: InsightDirection = direction === 'underweight' ? 'buy' : 'sell';

    insights.push({
      id: `target_drift_${holding.id}`,
      type: 'target_drift',
      severity,
      direction: insightDirection,
      title: `${holding.symbol} is ${direction} by ${driftPct}%`,
      explanation: `Current weight ${actualPct}% vs target ${targetPct}%. Consider rebalancing.`,
      metrics: {
        current: actualPct,
        target: targetPct,
        drift: driftPct,
      },
      action: 'Review Rebalance',
      linkTo: '/rebalance',
    });
  }

  // ── 2. concentration_risk ─────────────────────────────────────────────────
  for (const holding of holdings) {
    const weightPct = holding.weight; // already 0–100 percent from Rust
    if (weightPct <= 30) continue;

    const severity: InsightSeverity = weightPct > 50 ? 'critical' : 'warning';

    insights.push({
      id: `concentration_risk_${holding.id}`,
      type: 'concentration_risk',
      severity,
      direction: 'sell' as InsightDirection,
      title: `${holding.symbol} dominates the portfolio`,
      explanation: `${holding.symbol} represents ${weightPct.toFixed(1)}% of portfolio — consider diversifying.`,
      metrics: {
        weight: weightPct.toFixed(1),
      },
      action: 'View Holdings',
      linkTo: '/holdings',
    });
  }

  // ── 3. idle_cash ──────────────────────────────────────────────────────────
  const cashHoldings = holdings.filter((h) => h.assetType === 'cash');
  const totalCash = cashHoldings.reduce((sum, h) => sum + h.marketValueCad, 0);
  const cashPct = (totalCash / snapshot.totalValue) * 100;

  if (cashPct > 20) {
    insights.push({
      id: 'idle_cash',
      type: 'idle_cash',
      severity: 'info',
      direction: 'buy' as InsightDirection,
      title: 'High cash allocation',
      explanation: `Cash represents ${cashPct.toFixed(1)}% of portfolio — consider deploying.`,
      metrics: {
        cashPct: cashPct.toFixed(1),
      },
      action: 'View Holdings',
      linkTo: '/holdings',
    });
  }

  // ── 4. missing_targets ────────────────────────────────────────────────────
  const withoutTarget = holdings.filter((h) => !h.targetWeight || h.targetWeight <= 0);
  if (withoutTarget.length > holdings.length / 2) {
    insights.push({
      id: 'missing_targets',
      type: 'missing_targets',
      severity: 'info',
      direction: 'review' as InsightDirection,
      title: 'Target weights not set',
      explanation: `${withoutTarget.length} of ${holdings.length} holdings have no target weight. Set targets to enable rebalancing recommendations.`,
      metrics: {
        missing: withoutTarget.length,
        total: holdings.length,
      },
      action: 'Set Targets',
      linkTo: '/rebalance',
    });
  }

  // ── Sort: critical → warning → info, then by estimated impact (weight desc) ──
  return [...insights].sort((a, b) => {
    const severityDiff = SEVERITY_RANK[a.severity] - SEVERITY_RANK[b.severity];
    if (severityDiff !== 0) return severityDiff;
    // Secondary: holding weight (extracted from metrics if available, else 0)
    const aWeight = typeof a.metrics?.weight === 'string' ? parseFloat(a.metrics.weight) : 0;
    const bWeight = typeof b.metrics?.weight === 'string' ? parseFloat(b.metrics.weight) : 0;
    return bWeight - aWeight;
  });
}

// ─── Hook ─────────────────────────────────────────────────────────────────

export function useActionInsights(
  snapshot: PortfolioSnapshot | null,
  holdings: HoldingWithPrice[]
): ActionInsight[] {
  return useMemo(() => {
    if (!snapshot) return [];
    return buildInsights(snapshot, holdings);
  }, [snapshot, holdings]);
}
