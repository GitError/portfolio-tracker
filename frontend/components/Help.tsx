import { BookOpen } from 'lucide-react';

interface Term {
  term: string;
  definition: string;
}

interface HelpSection {
  title: string;
  terms: Term[];
}

const HELP_SECTIONS: HelpSection[] = [
  {
    title: 'Portfolio Basics',
    terms: [
      {
        term: 'Holding',
        definition: 'An individual asset (stock, ETF, crypto, or cash) in your portfolio.',
      },
      {
        term: 'Cost Basis',
        definition:
          'The original purchase price per unit of an asset. Used to calculate gains and losses.',
      },
      {
        term: 'Market Value',
        definition: 'Current price × quantity, converted to your base currency.',
      },
      {
        term: 'Asset Type',
        definition:
          'The category of an asset: Stock, ETF, Crypto, or Cash. Used to apply asset-specific shocks in stress tests.',
      },
      {
        term: 'Quantity',
        definition:
          'The number of units of an asset you hold (shares, coins, or currency amount for cash).',
      },
    ],
  },
  {
    title: 'Performance & Returns',
    terms: [
      {
        term: 'Gain / Loss',
        definition:
          'The difference between current market value and your cost basis. Positive = profit, negative = loss.',
      },
      {
        term: 'Daily P&L',
        definition:
          "Change in portfolio value since market open, based on each asset's daily price change percentage.",
      },
      {
        term: 'Return (%)',
        definition: 'Gain or loss expressed as a percentage of your total cost basis.',
      },
      {
        term: 'Weight',
        definition: "A holding's market value as a percentage of your total portfolio value.",
      },
      {
        term: 'Daily Change %',
        definition: "The percentage change in an asset's price since the previous market close.",
      },
    ],
  },
  {
    title: 'Allocation & Rebalancing',
    terms: [
      {
        term: 'Target Weight',
        definition: 'Your desired allocation percentage for a holding or asset class.',
      },
      {
        term: 'Drift',
        definition:
          "The difference between a holding's current weight and its target weight. Large drift signals a need to rebalance.",
      },
      {
        term: 'Rebalance',
        definition:
          'Adjusting holdings by buying or selling to bring allocations back to target weights.',
      },
      {
        term: 'Over-weight',
        definition:
          'A holding whose current weight exceeds its target weight, suggesting a potential sell or trim.',
      },
      {
        term: 'Under-weight',
        definition:
          'A holding whose current weight is below its target weight, suggesting a potential buy or add.',
      },
    ],
  },
  {
    title: 'Risk & Stress Testing',
    terms: [
      {
        term: 'Drawdown',
        definition:
          'The peak-to-trough decline in portfolio value during a specific period. Measures downside risk.',
      },
      {
        term: 'Volatility',
        definition:
          'The degree of variation in portfolio returns over time. Higher volatility = higher risk.',
      },
      {
        term: 'Stress Scenario',
        definition:
          'A hypothetical market shock applied to your holdings to estimate the potential impact on your portfolio value.',
      },
      {
        term: 'Shock (%)',
        definition:
          'The percentage change applied to an asset class or FX rate in a stress scenario (e.g., −20% for stocks).',
      },
      {
        term: 'Resilience Score',
        definition:
          'A composite metric reflecting portfolio diversification, concentration risk, and cash buffer strength.',
      },
    ],
  },
  {
    title: 'Currency & FX',
    terms: [
      {
        term: 'Base Currency',
        definition:
          'The currency in which your total portfolio value is displayed. Default is CAD.',
      },
      {
        term: 'FX Rate',
        definition:
          "Exchange rate between a holding's native currency and your base currency, used to convert market values.",
      },
      {
        term: 'FX Exposure',
        definition:
          'The portion of your portfolio held in assets denominated in a currency other than your base currency.',
      },
      {
        term: 'Cost Value (CAD)',
        definition:
          'Your cost basis converted to your base currency using the FX rate at the time of calculation.',
      },
    ],
  },
  {
    title: 'Alerts & Dividends',
    terms: [
      {
        term: 'Price Alert',
        definition:
          "A notification triggered when an asset's price crosses a threshold you configure.",
      },
      {
        term: 'Alert Condition',
        definition:
          'The rule that triggers an alert — e.g., "price above $150" or "price below $100".',
      },
      {
        term: 'Dividend',
        definition:
          'A payment made by a company to shareholders, recorded in Dividend History for income tracking.',
      },
      {
        term: 'Yield',
        definition: 'Annual dividend payment expressed as a percentage of the current share price.',
      },
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
            Help &amp; Glossary
          </div>
          <div
            style={{
              fontFamily: 'var(--font-sans)',
              fontSize: 12,
              color: 'var(--text-secondary)',
              marginTop: 3,
            }}
          >
            Definitions for terms used throughout Portfolio Tracker.
          </div>
        </div>
      </div>

      {/* Sections */}
      {HELP_SECTIONS.map((section) => (
        <div key={section.title} style={SECTION_CARD}>
          <div style={SECTION_HEADING}>{section.title}</div>
          <div style={TERM_GRID}>
            {section.terms.map(({ term, definition }) => (
              <div key={term} style={TERM_ROW}>
                <span style={TERM_LABEL}>{term}</span>
                <span style={TERM_DEF}>{definition}</span>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
