import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, act, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { Analytics } from '../Analytics';
import type { PortfolioAnalytics } from '../../types/portfolio';

// Initialize i18n (Analytics uses useTranslation)
import i18next from '../../lib/i18n';

const mockAnalytics: PortfolioAnalytics = {
  metadata: [{ symbol: 'AAPL', beta: 1.2, peRatio: 25.5 }],
  riskMetrics: {
    weightedBeta: 1234.5,
    portfolioYield: 0.05,
    largestPositionWeight: 12.34,
    concentrationHhi: 1500,
  },
  sectorBreakdown: [],
  countryBreakdown: [],
};

vi.mock('../../lib/tauri', () => ({
  isTauri: () => true,
  tauriInvoke: () => Promise.resolve(mockAnalytics),
}));

afterEach(async () => {
  await act(async () => {
    await i18next.changeLanguage('en');
  });
});

function renderAnalytics() {
  return render(
    <MemoryRouter>
      <Analytics />
    </MemoryRouter>
  );
}

describe('Analytics risk metric formatting', () => {
  it('renders the weighted beta with locale-aware separators in English', async () => {
    renderAnalytics();
    screen.getByRole('button', { name: /load analytics/i }).click();

    await waitFor(() => expect(screen.getByText('1,234.50')).toBeTruthy());
  });

  it('renders the weighted beta with German locale separators (comma decimal, period thousands)', async () => {
    await act(async () => {
      await i18next.changeLanguage('de');
    });

    renderAnalytics();
    screen.getByRole('button').click();

    await waitFor(() => expect(screen.getByText('1.234,50')).toBeTruthy());
    // Guard against the pre-fix behavior (raw toFixed() always uses '.' regardless of locale).
    expect(screen.queryByText('1234.50')).toBeNull();
  });
});
