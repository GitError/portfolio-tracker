import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  AlertTriangle,
  AlertCircle,
  Info,
  CheckCircle,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import type { ActionInsight, InsightDirection, InsightSeverity } from '../types/portfolio';

interface ActionCenterProps {
  insights: ActionInsight[];
}

// ─── Severity helpers ──────────────────────────────────────────────────────

const SEVERITY_COLOR: Record<InsightSeverity, string> = {
  critical: 'var(--color-loss)',
  warning: 'var(--color-warning)',
  info: 'var(--color-accent)',
};

const SEVERITY_BG: Record<InsightSeverity, string> = {
  critical: 'rgba(255, 71, 87, 0.08)',
  warning: 'rgba(251, 191, 36, 0.08)',
  info: 'rgba(59, 130, 246, 0.08)',
};

function SeverityIcon({ severity, size = 15 }: { severity: InsightSeverity; size?: number }) {
  const color = SEVERITY_COLOR[severity];
  if (severity === 'critical') return <AlertCircle size={size} color={color} />;
  if (severity === 'warning') return <AlertTriangle size={size} color={color} />;
  return <Info size={size} color={color} />;
}

// ─── Single insight card ──────────────────────────────────────────────────

interface InsightCardProps {
  insight: ActionInsight;
}

function InsightCard({ insight }: InsightCardProps) {
  const navigate = useNavigate();
  const color = SEVERITY_COLOR[insight.severity];
  const bg = SEVERITY_BG[insight.severity];

  function handleCta() {
    if (insight.linkTo) {
      navigate(insight.linkTo);
    }
  }

  return (
    <div
      style={{
        display: 'flex',
        gap: 12,
        padding: '12px 14px',
        background: bg,
        border: `1px solid ${color}26`,
        borderLeft: `3px solid ${color}`,
      }}
    >
      {/* Icon */}
      <div style={{ paddingTop: 1, flexShrink: 0 }}>
        <SeverityIcon severity={insight.severity} size={14} />
      </div>

      {/* Content */}
      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          style={{
            fontFamily: 'var(--font-mono)',
            fontSize: 12,
            fontWeight: 600,
            color: 'var(--text-primary)',
            marginBottom: 3,
          }}
        >
          {insight.title}
        </div>
        <div
          style={{
            fontFamily: 'var(--font-sans)',
            fontSize: 11,
            color: 'var(--text-secondary)',
            lineHeight: 1.5,
          }}
        >
          {insight.explanation}
        </div>
      </div>

      {/* CTA */}
      {insight.action && insight.linkTo && (
        <div style={{ flexShrink: 0, display: 'flex', alignItems: 'center' }}>
          <button
            onClick={handleCta}
            style={{
              background: 'transparent',
              border: `1px solid ${color}66`,
              borderRadius: 2,
              padding: '3px 8px',
              fontFamily: 'var(--font-mono)',
              fontSize: 10,
              color: color,
              cursor: 'pointer',
              textTransform: 'uppercase',
              letterSpacing: '0.06em',
              whiteSpace: 'nowrap',
            }}
          >
            {insight.action}
          </button>
        </div>
      )}
    </div>
  );
}

// ─── Action Center panel ──────────────────────────────────────────────────

const DIRECTION_COLOR: Record<InsightDirection, string> = {
  buy: 'var(--color-accent)',
  sell: 'var(--color-loss)',
  review: 'var(--text-secondary)',
};

const DIRECTION_BG: Record<InsightDirection, string> = {
  buy: 'rgba(59, 130, 246, 0.12)',
  sell: 'rgba(255, 71, 87, 0.12)',
  review: 'rgba(107, 114, 128, 0.12)',
};

export function ActionCenter({ insights }: ActionCenterProps) {
  const [expanded, setExpanded] = useState(false);

  const criticalCount = insights.filter((i) => i.severity === 'critical').length;
  const warningCount = insights.filter((i) => i.severity === 'warning').length;
  const buyCount = insights.filter((i) => i.direction === 'buy').length;
  const sellCount = insights.filter((i) => i.direction === 'sell').length;
  const reviewCount = insights.filter((i) => i.direction === 'review').length;

  return (
    <div
      style={{
        background: 'var(--bg-surface)',
        border: '1px solid var(--border-primary)',
        overflow: 'hidden',
      }}
    >
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '12px 16px',
          cursor: 'pointer',
          borderBottom: expanded ? '1px solid var(--border-subtle)' : 'none',
          userSelect: 'none',
        }}
        onClick={() => setExpanded((prev) => !prev)}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <span
            style={{
              fontSize: 11,
              color: 'var(--text-muted)',
              fontFamily: 'var(--font-mono)',
              textTransform: 'uppercase',
              letterSpacing: '0.08em',
            }}
          >
            Action Center
          </span>

          {/* Badge summary — severity + direction counts */}
          {insights.length > 0 && (
            <div style={{ display: 'flex', gap: 5 }}>
              {criticalCount > 0 && (
                <span
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: 3,
                    padding: '1px 5px',
                    background: 'rgba(255, 71, 87, 0.15)',
                    border: '1px solid rgba(255, 71, 87, 0.3)',
                    borderRadius: 2,
                    fontFamily: 'var(--font-mono)',
                    fontSize: 10,
                    color: 'var(--color-loss)',
                    fontWeight: 600,
                  }}
                >
                  <AlertCircle size={9} />
                  {criticalCount}
                </span>
              )}
              {warningCount > 0 && (
                <span
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: 3,
                    padding: '1px 5px',
                    background: 'rgba(251, 191, 36, 0.15)',
                    border: '1px solid rgba(251, 191, 36, 0.3)',
                    borderRadius: 2,
                    fontFamily: 'var(--font-mono)',
                    fontSize: 10,
                    color: 'var(--color-warning)',
                    fontWeight: 600,
                  }}
                >
                  <AlertTriangle size={9} />
                  {warningCount}
                </span>
              )}
              {/* Direction badges */}
              {(
                [
                  ['buy', buyCount],
                  ['sell', sellCount],
                  ['review', reviewCount],
                ] as [InsightDirection, number][]
              ).map(([dir, count]) =>
                count > 0 ? (
                  <span
                    key={dir}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      padding: '1px 5px',
                      background: DIRECTION_BG[dir],
                      border: `1px solid ${DIRECTION_COLOR[dir]}33`,
                      borderRadius: 2,
                      fontFamily: 'var(--font-mono)',
                      fontSize: 10,
                      color: DIRECTION_COLOR[dir],
                      fontWeight: 600,
                      textTransform: 'uppercase',
                      letterSpacing: '0.05em',
                    }}
                  >
                    {count} {dir}
                  </span>
                ) : null
              )}
            </div>
          )}
        </div>

        <div style={{ color: 'var(--text-muted)', display: 'flex', alignItems: 'center' }}>
          {expanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
        </div>
      </div>

      {/* Body */}
      {expanded && (
        <div>
          {insights.length === 0 ? (
            // Empty state
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 10,
                padding: '16px',
                color: 'var(--text-muted)',
              }}
            >
              <CheckCircle size={16} color="var(--color-gain)" />
              <span
                style={{
                  fontFamily: 'var(--font-sans)',
                  fontSize: 12,
                  color: 'var(--text-secondary)',
                }}
              >
                No actions needed — portfolio looks good
              </span>
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
              {insights.map((insight) => (
                <InsightCard key={insight.id} insight={insight} />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
