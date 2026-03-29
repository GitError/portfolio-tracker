// ─── Portfolio resilience summary panel for stress test ──────────────────────
import { formatCompact } from '../lib/format';
import type { PortfolioSnapshot } from '../types/portfolio';

const PANEL: React.CSSProperties = {
  background: 'var(--bg-surface)',
  border: '1px solid var(--border-primary)',
  padding: '20px',
};

const SECTION_TITLE: React.CSSProperties = {
  fontSize: 10,
  color: 'var(--text-muted)',
  fontFamily: 'var(--font-mono)',
  textTransform: 'uppercase',
  letterSpacing: '0.1em',
  marginBottom: 12,
  paddingBottom: 6,
  borderBottom: '1px solid var(--border-subtle)',
};

const STAT_CARD: React.CSSProperties = {
  background: 'var(--bg-surface)',
  border: '1px solid var(--border-primary)',
  padding: '14px 16px',
  flex: '1 1 0',
  minWidth: 160,
};

const STAT_LABEL: React.CSSProperties = {
  fontSize: 10,
  color: 'var(--text-muted)',
  fontFamily: 'var(--font-mono)',
  textTransform: 'uppercase',
  letterSpacing: '0.08em',
  marginBottom: 6,
};

const STAT_VALUE: React.CSSProperties = {
  fontSize: 20,
  fontFamily: 'var(--font-mono)',
  fontWeight: 700,
  color: 'var(--text-primary)',
  lineHeight: 1.2,
};

const STAT_SUB: React.CSSProperties = {
  fontSize: 11,
  fontFamily: 'var(--font-mono)',
  color: 'var(--text-secondary)',
  marginTop: 4,
};

// ─── Props ────────────────────────────────────────────────────────────────────
export interface ResilienceSummaryProps {
  portfolio: PortfolioSnapshot | null;
}

// ─── ResilienceSummary component ──────────────────────────────────────────────
export function ResilienceSummary({ portfolio }: ResilienceSummaryProps) {
  if (!portfolio || portfolio.holdings.length === 0) return null;

  const baseCurrency = portfolio.baseCurrency;

  // Largest single-holding risk by weight
  const largestHolding = [...portfolio.holdings].sort((a, b) => b.weight - a.weight)[0];

  // Max hit: holding with highest market value (worst case if it goes to zero)
  const maxHitHolding = [...portfolio.holdings].sort(
    (a, b) => b.marketValueCad - a.marketValueCad
  )[0];

  // Diversification score: 1 - HHI, normalized
  // Only non-cash holdings for diversification computation
  const nonCash = portfolio.holdings.filter((h) => h.assetType !== 'cash');
  const n = nonCash.length;
  let diversificationScore = 0;
  if (n > 1) {
    const totalNonCash = nonCash.reduce((sum, h) => sum + h.marketValueCad, 0);
    if (totalNonCash > 0) {
      const hhi = nonCash.reduce((sum, h) => {
        const w = h.marketValueCad / totalNonCash;
        return sum + w * w;
      }, 0);
      diversificationScore = ((1 - hhi) / (1 - 1 / n)) * 100;
    }
  } else if (n === 1) {
    diversificationScore = 0;
  }

  // FX exposure: % of portfolio in non-base-currency holdings
  const fxExposureValue = portfolio.holdings
    .filter((h) => h.currency.toUpperCase() !== baseCurrency.toUpperCase())
    .reduce((sum, h) => sum + h.marketValueCad, 0);
  const fxExposurePct =
    portfolio.totalValue > 0 ? (fxExposureValue / portfolio.totalValue) * 100 : 0;

  // Cash buffer: % of portfolio in cash positions
  const cashValue = portfolio.holdings
    .filter((h) => h.assetType === 'cash')
    .reduce((sum, h) => sum + h.marketValueCad, 0);
  const cashPct = portfolio.totalValue > 0 ? (cashValue / portfolio.totalValue) * 100 : 0;

  return (
    <div style={{ marginTop: 1 }}>
      <div
        style={{
          ...PANEL,
          paddingBottom: 16,
          background: 'var(--bg-surface)',
        }}
      >
        <div style={SECTION_TITLE}>Portfolio Resilience</div>
        <div
          style={{ display: 'flex', gap: 1, flexWrap: 'wrap', background: 'var(--border-primary)' }}
        >
          {/* Largest single-holding risk */}
          <div style={STAT_CARD}>
            <div style={STAT_LABEL}>Largest Position</div>
            <div style={STAT_VALUE}>
              {largestHolding ? `${(largestHolding.weight * 100).toFixed(1)}%` : '—'}
            </div>
            <div style={STAT_SUB}>{largestHolding ? largestHolding.symbol : '—'} of portfolio</div>
          </div>

          {/* Max hit from one holding */}
          <div style={STAT_CARD}>
            <div style={STAT_LABEL}>Max Single-Holding Loss</div>
            <div style={{ ...STAT_VALUE, color: 'var(--color-loss)' }}>
              {maxHitHolding ? formatCompact(maxHitHolding.marketValueCad) : '—'}
            </div>
            <div style={STAT_SUB}>{maxHitHolding ? `${maxHitHolding.symbol} at zero` : '—'}</div>
          </div>

          {/* Diversification score */}
          <div style={STAT_CARD}>
            <div style={STAT_LABEL}>Diversification Score</div>
            <div
              style={{
                ...STAT_VALUE,
                color:
                  diversificationScore >= 70
                    ? 'var(--color-gain)'
                    : diversificationScore >= 40
                      ? 'var(--color-warning)'
                      : 'var(--color-loss)',
              }}
            >
              {n > 0 ? `${diversificationScore.toFixed(0)} / 100` : '—'}
            </div>
            <div style={STAT_SUB}>
              {n > 0
                ? diversificationScore >= 70
                  ? 'Well diversified'
                  : diversificationScore >= 40
                    ? 'Moderate concentration'
                    : 'Highly concentrated'
                : 'No non-cash holdings'}
            </div>
          </div>

          {/* FX exposure */}
          <div style={STAT_CARD}>
            <div style={STAT_LABEL}>Foreign Currency Exposure</div>
            <div style={STAT_VALUE}>{fxExposurePct.toFixed(1)}%</div>
            <div style={STAT_SUB}>
              {formatCompact(fxExposureValue)} in non-{baseCurrency}
            </div>
          </div>

          {/* Cash buffer */}
          <div style={STAT_CARD}>
            <div style={STAT_LABEL}>Cash Buffer</div>
            <div
              style={{
                ...STAT_VALUE,
                color:
                  cashPct >= 5
                    ? 'var(--color-gain)'
                    : cashPct > 0
                      ? 'var(--color-warning)'
                      : 'var(--text-secondary)',
              }}
            >
              {cashPct.toFixed(1)}%
            </div>
            <div style={STAT_SUB}>{formatCompact(cashValue)} in cash positions</div>
          </div>
        </div>
      </div>
    </div>
  );
}
