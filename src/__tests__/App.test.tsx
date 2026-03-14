import { render } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import App from '../App';

// Mock Tauri — not available in jsdom
vi.mock('../hooks/usePortfolio', () => ({
  usePortfolio: () => ({
    portfolio: null,
    holdings: [],
    loading: true,
    error: null,
    refreshPrices: vi.fn(),
    addHolding: vi.fn(),
    updateHolding: vi.fn(),
    deleteHolding: vi.fn(),
  }),
}));

describe('App', () => {
  it('mounts without crashing', () => {
    const { container } = render(<App />);
    expect(container).toBeTruthy();
  });

  it('renders the sidebar', () => {
    render(<App />);
    // Sidebar always present in Layout
    expect(document.querySelector('nav') ?? document.querySelector('aside')).toBeTruthy();
  });

  it('shows Portfolio Tracker as document title when no portfolio loaded', () => {
    render(<App />);
    expect(document.title).toBe('Portfolio Tracker');
  });
});

describe('format utilities', () => {
  it('formatCurrency formats positive values', async () => {
    const { formatCurrency } = await import('../lib/format');
    expect(formatCurrency(1234.56, 'CAD')).toContain('1,234.56');
  });

  it('formatPercent includes sign', async () => {
    const { formatPercent } = await import('../lib/format');
    expect(formatPercent(12.34)).toBe('+12.34%');
    expect(formatPercent(-5.67)).toBe('-5.67%');
  });

  it('formatCompact abbreviates large numbers', async () => {
    const { formatCompact } = await import('../lib/format');
    expect(formatCompact(152300)).toContain('K');
    expect(formatCompact(1200000)).toContain('M');
  });
});
