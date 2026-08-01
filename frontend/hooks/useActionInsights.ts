import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import i18next from '../lib/i18n';
import { formatNumber } from '../lib/format';
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
  holdings: HoldingWithPrice[],
  t: TFunction = i18next.t,
  locale: string = i18next.language
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
    // Plain (non-localized) decimal strings for metrics — parsed via parseFloat() during sorting.
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
      title: t('insights.targetDrift.title', {
        symbol: holding.symbol,
        direction: t(`insights.targetDrift.${direction}`),
        pct: formatNumber(drift, 1, locale),
      }),
      explanation: t('insights.targetDrift.explanation', {
        actual: formatNumber(holding.weight, 1, locale),
        target: formatNumber(holding.targetWeight, 1, locale),
      }),
      metrics: {
        current: actualPct,
        target: targetPct,
        drift: driftPct,
      },
      action: t('insights.targetDrift.action'),
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
      title: t('insights.concentrationRisk.title', { symbol: holding.symbol }),
      explanation: t('insights.concentrationRisk.explanation', {
        symbol: holding.symbol,
        weight: formatNumber(weightPct, 1, locale),
      }),
      metrics: {
        weight: weightPct.toFixed(1),
      },
      action: t('insights.concentrationRisk.action'),
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
      title: t('insights.idleCash.title'),
      explanation: t('insights.idleCash.explanation', {
        pct: formatNumber(cashPct, 1, locale),
      }),
      metrics: {
        cashPct: cashPct.toFixed(1),
      },
      action: t('insights.idleCash.action'),
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
      title: t('insights.missingTargets.title'),
      explanation: t('insights.missingTargets.explanation', {
        missing: withoutTarget.length,
        total: holdings.length,
      }),
      metrics: {
        missing: withoutTarget.length,
        total: holdings.length,
      },
      action: t('insights.missingTargets.action'),
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
  const { t, i18n } = useTranslation();
  return useMemo(() => {
    if (!snapshot) return [];
    return buildInsights(snapshot, holdings, t, i18n.language);
  }, [snapshot, holdings, t, i18n.language]);
}
