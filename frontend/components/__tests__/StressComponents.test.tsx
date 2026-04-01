import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';

import '../../lib/i18n';

// ─── ResilienceSummary ────────────────────────────────────────────────────────
import { ResilienceSummary } from '../ResilienceSummary';
// ─── StressResultsTable ───────────────────────────────────────────────────────
import { StressResultsTable } from '../StressResultsTable';
// ─── PresetScenarioSelector ───────────────────────────────────────────────────
import { PresetScenarioSelector } from '../PresetScenarioSelector';

import type { PortfolioSnapshot, HoldingWithPrice, StressResult } from '../../types/portfolio';

// ─── Helpers ──────────────────────────────────────────────────────────────────
function makeHolding(
  id: string,
  symbol: string,
  assetType: HoldingWithPrice['assetType'],
  currency: string,
  value: number,
  weight: number
): HoldingWithPrice {
  return {
    id,
    symbol,
    name: symbol,
    assetType,
    account: 'taxable',
    quantity: 1,
    costBasis: value,
    currency,
    exchange: '',
    targetWeight: 0,
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
    indicatedAnnualDividend: null,
    indicatedAnnualDividendCurrency: null,
    dividendFrequency: null,
    maturityDate: null,
    currentPrice: value,
    currentPriceCad: value,
    marketValueCad: value,
    costValueCad: value,
    gainLoss: 0,
    gainLossPercent: 0,
    weight,
    targetValue: 0,
    targetDeltaValue: 0,
    targetDeltaPercent: 0,
    dailyChangePercent: 0,
    fxStale: false,
  };
}

function makeSnapshot(holdings: HoldingWithPrice[]): PortfolioSnapshot {
  const total = holdings.reduce((s, h) => s + h.marketValueCad, 0);
  return {
    holdings,
    totalValue: total,
    totalCost: total,
    totalGainLoss: 0,
    totalGainLossPercent: 0,
    dailyPnl: 0,
    lastUpdated: '2024-01-01T00:00:00Z',
    baseCurrency: 'CAD',
    totalTargetWeight: 0,
    targetCashDelta: 0,
    realizedGains: 0,
    annualDividendIncome: 0,
  };
}

// ─── ResilienceSummary tests ──────────────────────────────────────────────────
describe('ResilienceSummary', () => {
  it('renders nothing when portfolio is null', () => {
    const { container } = render(<ResilienceSummary portfolio={null} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders nothing when portfolio has no holdings', () => {
    const { container } = render(<ResilienceSummary portfolio={makeSnapshot([])} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders the Portfolio Resilience section title', () => {
    const holdings = [
      makeHolding('1', 'AAPL', 'stock', 'CAD', 10000, 70),
      makeHolding('2', 'CAD-CASH', 'cash', 'CAD', 3000, 30),
    ];
    render(<ResilienceSummary portfolio={makeSnapshot(holdings)} />);
    expect(screen.getByText(/portfolio resilience/i)).toBeTruthy();
  });

  it('shows the largest position symbol', () => {
    const holdings = [
      makeHolding('1', 'AAPL', 'stock', 'CAD', 8000, 80),
      makeHolding('2', 'VFV', 'etf', 'CAD', 2000, 20),
    ];
    render(<ResilienceSummary portfolio={makeSnapshot(holdings)} />);
    // Symbol appears at least once (e.g. in "AAPL of portfolio" sub-label)
    expect(screen.getAllByText(/AAPL/).length).toBeGreaterThan(0);
  });

  it('shows cash buffer stats', () => {
    const holdings = [
      makeHolding('1', 'AAPL', 'stock', 'CAD', 9000, 90),
      makeHolding('2', 'CAD-CASH', 'cash', 'CAD', 1000, 10),
    ];
    render(<ResilienceSummary portfolio={makeSnapshot(holdings)} />);
    expect(screen.getByText(/cash buffer/i)).toBeTruthy();
  });

  it('shows "Highly concentrated" when single non-cash holding', () => {
    const holdings = [makeHolding('1', 'AAPL', 'stock', 'CAD', 10000, 100)];
    render(<ResilienceSummary portfolio={makeSnapshot(holdings)} />);
    expect(screen.getByText(/highly concentrated/i)).toBeTruthy();
  });
});

// ─── StressResultsTable tests ─────────────────────────────────────────────────
describe('StressResultsTable', () => {
  const stressResult: StressResult = {
    scenario: 'Bear Market',
    currentValue: 15000,
    stressedValue: 12000,
    totalImpact: -3000,
    totalImpactPercent: -20,
    holdingBreakdown: [
      {
        holdingId: '1',
        symbol: 'AAPL',
        name: 'Apple',
        currentValue: 10000,
        stressedValue: 8000,
        impact: -2000,
        shockApplied: -0.2,
      },
      {
        holdingId: '2',
        symbol: 'BTC',
        name: 'Bitcoin',
        currentValue: 5000,
        stressedValue: 4000,
        impact: -1000,
        shockApplied: -0.2,
      },
    ],
  };

  const holdings = [
    makeHolding('1', 'AAPL', 'stock', 'CAD', 10000, 66.7),
    makeHolding('2', 'BTC', 'crypto', 'CAD', 5000, 33.3),
  ];

  it('renders the Breakdown panel', () => {
    render(<StressResultsTable result={stressResult} holdings={holdings} baseCurrency="CAD" />);
    expect(screen.getByText(/breakdown/i)).toBeTruthy();
  });

  it('renders holding symbols in the table', () => {
    render(<StressResultsTable result={stressResult} holdings={holdings} baseCurrency="CAD" />);
    expect(screen.getByText('AAPL')).toBeTruthy();
    expect(screen.getByText('BTC')).toBeTruthy();
  });

  it('renders column headers including base currency', () => {
    render(<StressResultsTable result={stressResult} holdings={holdings} baseCurrency="CAD" />);
    expect(screen.getByText(/Current Value \(CAD\)/i)).toBeTruthy();
  });

  it('renders shock percentage for each holding', () => {
    render(<StressResultsTable result={stressResult} holdings={holdings} baseCurrency="CAD" />);
    // -20% shock should appear twice
    const shockCells = screen.getAllByText(/-20/);
    expect(shockCells.length).toBeGreaterThan(0);
  });
});

// ─── PresetScenarioSelector tests ────────────────────────────────────────────
describe('PresetScenarioSelector', () => {
  const presetNames = ['Bear Market', 'Crypto Winter', 'Custom'];
  const scenarioInfo = [
    {
      name: 'Bear Market',
      description: 'A severe market downturn.',
      historicalParallel: '2008',
      shocks: { stock: -0.2 },
    },
    {
      name: 'Crypto Winter',
      description: 'Crypto markets collapse.',
      historicalParallel: '2022',
      shocks: { crypto: -0.5 },
    },
  ];

  it('renders the preset scenario label', () => {
    render(
      <PresetScenarioSelector
        presetName="Bear Market"
        presetNames={presetNames}
        scenarioInfo={scenarioInfo}
        onSelect={vi.fn()}
        onInfoOpen={vi.fn()}
      />
    );
    expect(screen.getByText(/preset scenario/i)).toBeTruthy();
  });

  it('shows the Info button for a known preset', () => {
    render(
      <PresetScenarioSelector
        presetName="Bear Market"
        presetNames={presetNames}
        scenarioInfo={scenarioInfo}
        onSelect={vi.fn()}
        onInfoOpen={vi.fn()}
      />
    );
    expect(screen.getByRole('button', { name: /info/i })).toBeTruthy();
  });

  it('hides the Info button when Custom is selected', () => {
    render(
      <PresetScenarioSelector
        presetName="Custom"
        presetNames={presetNames}
        scenarioInfo={scenarioInfo}
        onSelect={vi.fn()}
        onInfoOpen={vi.fn()}
      />
    );
    expect(screen.queryByRole('button', { name: /info/i })).toBeNull();
  });

  it('calls onInfoOpen when Info button is clicked', async () => {
    const onInfoOpen = vi.fn();
    render(
      <PresetScenarioSelector
        presetName="Bear Market"
        presetNames={presetNames}
        scenarioInfo={scenarioInfo}
        onSelect={vi.fn()}
        onInfoOpen={onInfoOpen}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /info/i }));
    expect(onInfoOpen).toHaveBeenCalledOnce();
  });

  it('shows scenario description for known preset', () => {
    render(
      <PresetScenarioSelector
        presetName="Bear Market"
        presetNames={presetNames}
        scenarioInfo={scenarioInfo}
        onSelect={vi.fn()}
        onInfoOpen={vi.fn()}
      />
    );
    expect(screen.getByText(/A severe market downturn/)).toBeTruthy();
  });
});
