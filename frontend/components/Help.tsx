import { BookOpen } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface Term {
  termKey: string;
  defKey: string;
}

interface HelpSection {
  titleKey: string;
  terms: Term[];
}

const HELP_SECTIONS: HelpSection[] = [
  {
    titleKey: 'help.portfolioBasics',
    terms: [
      { termKey: 'help.term.holding', defKey: 'help.def.holding' },
      { termKey: 'help.term.costBasis', defKey: 'help.def.costBasis' },
      { termKey: 'help.term.marketValue', defKey: 'help.def.marketValue' },
      { termKey: 'help.term.assetType', defKey: 'help.def.assetType' },
      { termKey: 'help.term.quantity', defKey: 'help.def.quantity' },
    ],
  },
  {
    titleKey: 'help.performanceReturns',
    terms: [
      { termKey: 'help.term.gainLoss', defKey: 'help.def.gainLoss' },
      { termKey: 'help.term.dailyPnl', defKey: 'help.def.dailyPnl' },
      { termKey: 'help.term.returnPct', defKey: 'help.def.returnPct' },
      { termKey: 'help.term.weight', defKey: 'help.def.weight' },
      { termKey: 'help.term.dailyChangePct', defKey: 'help.def.dailyChangePct' },
    ],
  },
  {
    titleKey: 'help.allocationRebalancing',
    terms: [
      { termKey: 'help.term.targetWeight', defKey: 'help.def.targetWeight' },
      { termKey: 'help.term.drift', defKey: 'help.def.drift' },
      { termKey: 'help.term.rebalance', defKey: 'help.def.rebalance' },
      { termKey: 'help.term.overweight', defKey: 'help.def.overweight' },
      { termKey: 'help.term.underweight', defKey: 'help.def.underweight' },
    ],
  },
  {
    titleKey: 'help.riskStressTesting',
    terms: [
      { termKey: 'help.term.drawdown', defKey: 'help.def.drawdown' },
      { termKey: 'help.term.volatility', defKey: 'help.def.volatility' },
      { termKey: 'help.term.stressScenario', defKey: 'help.def.stressScenario' },
      { termKey: 'help.term.shockPct', defKey: 'help.def.shockPct' },
      { termKey: 'help.term.resilienceScore', defKey: 'help.def.resilienceScore' },
    ],
  },
  {
    titleKey: 'help.currencyFx',
    terms: [
      { termKey: 'help.term.baseCurrency', defKey: 'help.def.baseCurrency' },
      { termKey: 'help.term.fxRate', defKey: 'help.def.fxRate' },
      { termKey: 'help.term.fxExposure', defKey: 'help.def.fxExposure' },
      { termKey: 'help.term.costValueCad', defKey: 'help.def.costValueCad' },
    ],
  },
  {
    titleKey: 'help.alertsDividends',
    terms: [
      { termKey: 'help.term.priceAlert', defKey: 'help.def.priceAlert' },
      { termKey: 'help.term.alertCondition', defKey: 'help.def.alertCondition' },
      { termKey: 'help.term.dividend', defKey: 'help.def.dividend' },
      { termKey: 'help.term.yield', defKey: 'help.def.yield' },
    ],
  },
];

const PAGE_WRAPPER: React.CSSProperties = {
  maxWidth: 960,
  margin: '0 auto',
};

const PAGE_HEADER: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 12,
  marginBottom: 28,
  paddingBottom: 20,
  borderBottom: '1px solid var(--border-primary)',
};

const SECTION_CARD: React.CSSProperties = {
  background: 'var(--bg-surface)',
  border: '1px solid var(--border-primary)',
  padding: '20px 24px',
  marginBottom: 1,
};

const SECTION_HEADING: React.CSSProperties = {
  fontSize: 10,
  fontFamily: 'var(--font-mono)',
  textTransform: 'uppercase',
  letterSpacing: '0.12em',
  color: 'var(--text-secondary)',
  marginBottom: 16,
  paddingBottom: 8,
  borderBottom: '1px solid var(--border-subtle)',
};

const TERM_GRID: React.CSSProperties = {
  display: 'grid',
  gridTemplateColumns: 'repeat(auto-fill, minmax(340px, 1fr))',
  gap: '12px 24px',
};

const TERM_ROW: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: 3,
};

const TERM_LABEL: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: 12,
  fontWeight: 600,
  color: 'var(--text-primary)',
};

const TERM_DEF: React.CSSProperties = {
  fontFamily: 'var(--font-sans)',
  fontSize: 12,
  color: 'var(--text-secondary)',
  lineHeight: 1.55,
};

export function Help() {
  const { t } = useTranslation();

  return (
    <div style={PAGE_WRAPPER}>
      {/* Page header */}
      <div style={PAGE_HEADER}>
        <BookOpen size={20} style={{ color: 'var(--color-accent)', flexShrink: 0 }} />
        <div>
          <div
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 16,
              fontWeight: 700,
              color: 'var(--text-primary)',
            }}
          >
            {t('help.title')}
          </div>
          <div
            style={{
              fontFamily: 'var(--font-sans)',
              fontSize: 12,
              color: 'var(--text-secondary)',
              marginTop: 3,
            }}
          >
            {t('help.subtitle')}
          </div>
        </div>
      </div>

      {/* Sections */}
      {HELP_SECTIONS.map((section) => (
        <div key={section.titleKey} style={SECTION_CARD}>
          <div style={SECTION_HEADING}>{t(section.titleKey)}</div>
          <div style={TERM_GRID}>
            {section.terms.map(({ termKey, defKey }) => (
              <div key={termKey} style={TERM_ROW}>
                <span style={TERM_LABEL}>{t(termKey)}</span>
                <span style={TERM_DEF}>{t(defKey)}</span>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
