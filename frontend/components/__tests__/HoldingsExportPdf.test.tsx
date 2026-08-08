import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { Holdings } from '../Holdings';
import { ToastProvider } from '../ui/Toast';

// Initialize i18n (Holdings uses useTranslation)
import i18next from '../../lib/i18n';

const mockExportPortfolioPdf = vi.fn();

vi.mock('../../hooks/usePortfolio', () => ({
  PortfolioProvider: ({ children }: { children: React.ReactNode }) => children,
  usePortfolio: () => ({
    portfolio: null,
    holdings: [],
    loading: false,
    error: null,
    failedSymbols: [],
    triggeredAlertIds: [],
    alertRefreshErrors: [],
    refreshPrices: vi.fn(),
    addHolding: vi.fn().mockResolvedValue({}),
    updateHolding: vi.fn().mockResolvedValue({}),
    deleteHolding: vi.fn().mockResolvedValue(undefined),
    importHoldingsCsv: vi.fn().mockResolvedValue({ imported: [], skipped: [], totalRows: 0 }),
    previewImportCsv: vi.fn().mockResolvedValue({ rows: [], readyCount: 0, skipCount: 0 }),
    exportHoldingsCsv: vi.fn().mockResolvedValue(''),
    exportPortfolioPdf: mockExportPortfolioPdf,
  }),
}));

beforeEach(async () => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
  localStorage.clear();
  mockExportPortfolioPdf.mockReset();
  // Pin language so English-text assertions don't break if the default locale changes.
  await i18next.changeLanguage('en');
});

function renderHoldings() {
  return render(
    <MemoryRouter>
      <ToastProvider>
        <Holdings />
      </ToastProvider>
    </MemoryRouter>
  );
}

function getExportPdfButton(): HTMLButtonElement {
  return screen.getByText('Export PDF').closest('button') as HTMLButtonElement;
}

describe('Holdings "Export PDF" button', () => {
  it('shows a success toast with the saved path on success', async () => {
    mockExportPortfolioPdf.mockResolvedValue('~/Downloads/portfolio-2026-08-08.pdf');
    renderHoldings();

    fireEvent.click(getExportPdfButton());

    await waitFor(() => {
      expect(screen.getByText('Exported to ~/Downloads/portfolio-2026-08-08.pdf')).toBeTruthy();
    });
    expect(mockExportPortfolioPdf).toHaveBeenCalledTimes(1);
  });

  it('shows an error toast with the failure message when the export rejects', async () => {
    mockExportPortfolioPdf.mockRejectedValue(new Error('Could not resolve home directory'));
    renderHoldings();

    fireEvent.click(getExportPdfButton());

    await waitFor(() => {
      expect(screen.getByText('Could not resolve home directory')).toBeTruthy();
    });
  });

  it('ignores extra clicks while an export is already in flight, then completes once', async () => {
    let resolveExport: (path: string) => void = () => {};
    mockExportPortfolioPdf.mockImplementation(
      () =>
        new Promise<string>((resolve) => {
          resolveExport = resolve;
        })
    );
    renderHoldings();
    const button = getExportPdfButton();

    fireEvent.click(button);
    fireEvent.click(button);
    fireEvent.click(button);

    // Still in flight — the two extra clicks must be a no-op.
    expect(mockExportPortfolioPdf).toHaveBeenCalledTimes(1);

    resolveExport('~/Downloads/portfolio-2026-08-08.pdf');

    await waitFor(() => {
      expect(screen.getByText('Exported to ~/Downloads/portfolio-2026-08-08.pdf')).toBeTruthy();
    });
    expect(mockExportPortfolioPdf).toHaveBeenCalledTimes(1);
  });
});
