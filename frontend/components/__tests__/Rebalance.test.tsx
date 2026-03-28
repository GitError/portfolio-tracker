import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { Rebalance } from '../Rebalance';
import type { RebalanceSuggestion } from '../../types/portfolio';

const mockGetRebalanceSuggestions = vi.fn();

vi.mock('../../lib/tauri', () => ({
  isTauri: () => false,
  tauriInvoke: (cmd: string, ...args: unknown[]) => {
    if (cmd === 'get_rebalance_suggestions') return mockGetRebalanceSuggestions(cmd, ...args);
    return Promise.resolve([]);
  },
}));

const MOCK_SUGGESTIONS: RebalanceSuggestion[] = [
  {
    holdingId: 'h1',
    symbol: 'AAPL',
    name: 'Apple Inc.',
    currentValueCad: 12000,
    targetValueCad: 10000,
    currentWeight: 60,
    targetWeight: 50,
    drift: 10,
    suggestedTradeCad: 2000,
    suggestedUnits: 10,
    currentPriceCad: 200,
  },
  {
    holdingId: 'h2',
    symbol: 'MSFT',
    name: 'Microsoft Corp.',
    currentValueCad: 8000,
    targetValueCad: 10000,
    currentWeight: 40,
    targetWeight: 50,
    drift: -10,
    suggestedTradeCad: -2000,
    suggestedUnits: -5,
    currentPriceCad: 400,
  },
];

beforeEach(() => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
  vi.clearAllMocks();
  // Default: return empty suggestions
  mockGetRebalanceSuggestions.mockResolvedValue([]);
});

function renderRebalance() {
  return render(
    <MemoryRouter>
      <Rebalance />
    </MemoryRouter>
  );
}

describe('Rebalance component smoke tests', () => {
  it('renders without crashing', async () => {
    const { container } = renderRebalance();
    expect(container).toBeTruthy();
    await waitFor(() => expect(mockGetRebalanceSuggestions).toHaveBeenCalled());
  });

  it('renders the Rebalance heading', async () => {
    renderRebalance();
    await waitFor(() => expect(mockGetRebalanceSuggestions).toHaveBeenCalled());
    expect(screen.getByText('Rebalance')).toBeTruthy();
  });

  it('shows empty state when there are no suggestions', async () => {
    mockGetRebalanceSuggestions.mockResolvedValue([]);
    renderRebalance();
    await waitFor(() => screen.getByText(/no rebalancing needed/i));
    expect(screen.getByText(/no rebalancing needed/i)).toBeTruthy();
  });

  it('shows hint about setting target weights in empty state', async () => {
    mockGetRebalanceSuggestions.mockResolvedValue([]);
    renderRebalance();
    await waitFor(() => screen.getByText(/target weights/i));
    expect(screen.getByText(/target weights/i)).toBeTruthy();
  });

  it('renders rebalance table when suggestions are present', async () => {
    mockGetRebalanceSuggestions.mockResolvedValue(MOCK_SUGGESTIONS);
    renderRebalance();
    await waitFor(() => screen.getByText('AAPL'));
    expect(screen.getByText('AAPL')).toBeTruthy();
    expect(screen.getByText('MSFT')).toBeTruthy();
  });

  it('shows Symbol column header when suggestions are present', async () => {
    mockGetRebalanceSuggestions.mockResolvedValue(MOCK_SUGGESTIONS);
    renderRebalance();
    await waitFor(() => screen.getByText('Symbol'));
    expect(screen.getByText('Symbol')).toBeTruthy();
  });

  it('shows drift threshold control', async () => {
    renderRebalance();
    await waitFor(() => expect(mockGetRebalanceSuggestions).toHaveBeenCalled());
    expect(screen.getByLabelText(/show drift/i)).toBeTruthy();
  });

  it('shows suggestion count summary when suggestions exist', async () => {
    mockGetRebalanceSuggestions.mockResolvedValue(MOCK_SUGGESTIONS);
    renderRebalance();
    await waitFor(() => screen.getByText('AAPL'));
    // Summary text: "2 holdings with drift > 5%"
    expect(screen.getByText(/holdings with drift/i)).toBeTruthy();
  });
});
