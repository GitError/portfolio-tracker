import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { Research } from '../Research';

// Initialize i18n (Research uses useTranslation)
import i18next from '../../lib/i18n';

let mockIsTauri = false;
const mockListWatchlists = vi.fn();
const mockListWatchlistItems = vi.fn();

vi.mock('../../lib/tauri', () => ({
  isTauri: () => mockIsTauri,
  getErrorMessage: (e: unknown) => (e instanceof Error ? e.message : String(e)),
  tauriInvoke: (cmd: string, args?: Record<string, unknown>) => {
    if (cmd === 'list_watchlists') return mockListWatchlists();
    if (cmd === 'list_watchlist_items') return mockListWatchlistItems(args);
    return Promise.resolve(null);
  },
}));

vi.mock('../../hooks/usePortfolio', () => ({
  usePortfolio: () => ({
    holdings: [],
    addHolding: vi.fn().mockResolvedValue({}),
  }),
}));

beforeEach(async () => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
  vi.clearAllMocks();
  mockIsTauri = false;
  mockListWatchlists.mockResolvedValue([]);
  mockListWatchlistItems.mockResolvedValue([]);
  await i18next.changeLanguage('en');
});

function renderResearch() {
  return render(
    <MemoryRouter>
      <Research />
    </MemoryRouter>
  );
}

describe('Research component', () => {
  it('renders the mock watchlist and its items in browser mode', async () => {
    renderResearch();
    await waitFor(() => screen.getByText(/growth ideas/i));
    await waitFor(() => screen.getByText('NVDA'));
    expect(screen.getByText('NVDA')).toBeTruthy();
    expect(screen.getByText('SHOP.TO')).toBeTruthy();
  });

  it('shows an empty state when there are no watchlists', async () => {
    mockIsTauri = true;
    mockListWatchlists.mockResolvedValue([]);
    renderResearch();
    await waitFor(() => screen.getByText(/no watchlists yet/i));
    expect(screen.getByText(/no watchlists yet/i)).toBeTruthy();
  });

  it('shows an empty-items state when the selected watchlist has no symbols', async () => {
    mockIsTauri = true;
    mockListWatchlists.mockResolvedValue([
      {
        id: 'w1',
        name: 'Empty List',
        createdAt: '2026-01-01T00:00:00Z',
        updatedAt: '2026-01-01T00:00:00Z',
      },
    ]);
    mockListWatchlistItems.mockResolvedValue([]);
    renderResearch();
    await waitFor(() => screen.getByText(/no symbols yet/i));
    expect(screen.getByText(/no symbols yet/i)).toBeTruthy();
  });

  it('shows a persistent error with a retry button when loading watchlists fails, and retry re-fetches', async () => {
    mockIsTauri = true;
    mockListWatchlists.mockRejectedValue(new Error('network down'));

    renderResearch();

    await waitFor(() => screen.getByText(/failed to load watchlists/i));
    const retryButton = screen.getByRole('button', { name: /retry/i });
    expect(retryButton).toBeTruthy();

    mockListWatchlists.mockResolvedValue([]);
    fireEvent.click(retryButton);

    await waitFor(() => expect(mockListWatchlists).toHaveBeenCalledTimes(2));
  });

  it('renders a per-row error state when a symbol snapshot failed to fetch', async () => {
    renderResearch();
    await waitFor(() => screen.getByText(/growth ideas/i));
    await waitFor(() =>
      expect(screen.getByText(/no quote data returned for badsym/i)).toBeTruthy()
    );
  });

  it('shows a stale indicator for a snapshot older than 15 minutes', async () => {
    renderResearch();
    await waitFor(() => screen.getByText(/growth ideas/i));
    // The symbol cell's grandparent is the row's grid container.
    const shopRow = screen.getByText('SHOP.TO').parentElement?.parentElement;
    expect(shopRow).toBeTruthy();
    expect(shopRow?.textContent).not.toMatch(/not fetched yet/i);
    // SHOP.TO's mock snapshot is 20 minutes old (isStale: true) — the stale
    // warning icon renders next to its "Last Updated" timestamp.
    expect(shopRow?.querySelector('svg')).toBeTruthy();
  });

  it('pre-fills the Add Holding modal with symbol/currency/name when "Add to Holdings" is clicked', async () => {
    renderResearch();
    await waitFor(() => screen.getByText(/growth ideas/i));
    await waitFor(() => screen.getByText('NVDA'));

    const addButtons = screen.getAllByTitle(/add to holdings/i);
    fireEvent.click(addButtons[0]!);

    await waitFor(() => expect(screen.getByDisplayValue('NVIDIA Corporation')).toBeTruthy());
  });
});
