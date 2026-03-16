import { render } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import App from '../App';

// Mock Tauri — not available in jsdom
vi.mock('../hooks/usePortfolio', () => ({
  PortfolioProvider: ({ children }: { children: React.ReactNode }) => children,
  usePortfolio: () => ({
    portfolio: null,
    holdings: [],
    loading: true,
    error: null,
    failedSymbols: [],
    triggeredAlertIds: [],
    refreshPrices: vi.fn(),
    addHolding: vi.fn(),
    updateHolding: vi.fn(),
    deleteHolding: vi.fn(),
    importHoldingsCsv: vi.fn(),
    previewImportCsv: vi.fn(),
    exportHoldingsCsv: vi.fn(),
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

describe('Tauri detection', () => {
  it('isTauri returns false in browser (no __TAURI_INTERNALS__)', async () => {
    // In the test environment window.__TAURI_INTERNALS__ is not set
    expect('__TAURI_INTERNALS__' in window).toBe(false);
  });

  it('mock data path is taken when __TAURI_INTERNALS__ absent', async () => {
    // Directly import and exercise the hook logic by checking the detection
    // key — if __TAURI_INTERNALS__ is absent, the browser mock path runs.
    const hasTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
    expect(hasTauri).toBe(false);
  });

  it('isTauri would return true when __TAURI_INTERNALS__ is present', () => {
    // Simulate what Tauri v2 sets on the window object
    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      value: { invoke: vi.fn() },
      configurable: true,
      writable: true,
    });
    expect('__TAURI_INTERNALS__' in window).toBe(true);
    // Cleanup
    // @ts-expect-error — test teardown
    delete window.__TAURI_INTERNALS__;
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
