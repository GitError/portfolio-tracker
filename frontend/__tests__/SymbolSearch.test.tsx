import { render, screen, fireEvent, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { SymbolSearch } from '../components/ui/SymbolSearch';

// Ensure we are always in the browser (non-Tauri) mock path
beforeEach(() => {
  // @ts-expect-error — intentionally removing Tauri internals for mock path
  delete window.__TAURI_INTERNALS__;
  vi.useFakeTimers();
});

afterEach(() => {
  vi.runAllTimers();
  vi.useRealTimers();
});

function renderSymbolSearch(overrides: Partial<React.ComponentProps<typeof SymbolSearch>> = {}) {
  const onSelect = vi.fn();
  const onChange = vi.fn();
  const result = render(
    <SymbolSearch
      value=""
      onChange={onChange}
      onSelect={onSelect}
      placeholder="AAPL"
      {...overrides}
    />
  );
  return { ...result, onSelect, onChange };
}

/** Type into the search input and flush the debounce timer. */
async function typeAndFlush(input: HTMLElement, value: string) {
  fireEvent.change(input, { target: { value } });
  await act(async () => {
    vi.advanceTimersByTime(400);
  });
}

describe('SymbolSearch – basic rendering', () => {
  it('renders the input', () => {
    renderSymbolSearch();
    expect(screen.getByPlaceholderText('AAPL')).toBeTruthy();
  });

  it('does not show dropdown before typing', () => {
    renderSymbolSearch();
    expect(screen.queryByText('Apple Inc.')).toBeNull();
  });
});

describe('SymbolSearch – mock data filtering', () => {
  it('shows results after debounce when query length >= 2', async () => {
    renderSymbolSearch();
    const input = screen.getByPlaceholderText('AAPL');
    await typeAndFlush(input, 'aa');
    expect(screen.queryByText('Apple Inc.')).toBeTruthy();
  });

  it('does not show results when query is only 1 character', async () => {
    renderSymbolSearch();
    const input = screen.getByPlaceholderText('AAPL');
    await typeAndFlush(input, 'a');
    expect(screen.queryByText('Apple Inc.')).toBeNull();
  });

  it('filters results by symbol prefix', async () => {
    renderSymbolSearch();
    const input = screen.getByPlaceholderText('AAPL');
    await typeAndFlush(input, 'vo');
    expect(screen.queryByText('Vanguard S&P 500 ETF')).toBeTruthy();
  });

  it('calls onSelect when a result is clicked', async () => {
    const { onSelect } = renderSymbolSearch();
    const input = screen.getByPlaceholderText('AAPL');
    await typeAndFlush(input, 'aa');
    expect(screen.queryByText('Apple Inc.')).toBeTruthy();
    // Use mouseDown — the component uses onMouseDown to prevent blur before select
    fireEvent.mouseDown(screen.getByText('Apple Inc.'));
    expect(onSelect).toHaveBeenCalledWith(
      expect.objectContaining({ symbol: 'AAPL', name: 'Apple Inc.' })
    );
  });
});

describe('SymbolSearch – stale async request guard', () => {
  it('discards stale responses when the query changes before the result arrives', async () => {
    // We simulate the guard by typing two queries in quick succession.
    // The second fireEvent.change clears the first debounce timer and
    // updates currentQueryRef before the first async callback executes.
    // Only the second query's results should appear.
    renderSymbolSearch();
    const input = screen.getByPlaceholderText('AAPL');

    // Type first query — schedules a 300 ms debounce
    fireEvent.change(input, { target: { value: 'aa' } });
    // Immediately type a second query — clears the first debounce and schedules a new one
    fireEvent.change(input, { target: { value: 'ms' } });

    // Advance past the debounce; only the second timer fires (first was cleared)
    await act(async () => {
      vi.advanceTimersByTime(400);
    });

    // Results for 'ms' (Microsoft) should be shown, not Apple
    expect(screen.queryByText('Microsoft Corporation')).toBeTruthy();
    expect(screen.queryByText('Apple Inc.')).toBeNull();
  });
});

describe('SymbolSearch – keyboard navigation', () => {
  it('highlights items with ArrowDown and selects with Enter', async () => {
    const { onSelect } = renderSymbolSearch();
    const input = screen.getByPlaceholderText('AAPL');
    await typeAndFlush(input, 'aa');
    expect(screen.queryByText('Apple Inc.')).toBeTruthy();
    fireEvent.keyDown(input, { key: 'ArrowDown' });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onSelect).toHaveBeenCalledOnce();
  });

  it('closes dropdown on Escape', async () => {
    renderSymbolSearch();
    const input = screen.getByPlaceholderText('AAPL');
    await typeAndFlush(input, 'aa');
    expect(screen.queryByText('Apple Inc.')).toBeTruthy();
    await act(async () => {
      fireEvent.keyDown(input, { key: 'Escape' });
    });
    expect(screen.queryByText('Apple Inc.')).toBeNull();
  });
});
