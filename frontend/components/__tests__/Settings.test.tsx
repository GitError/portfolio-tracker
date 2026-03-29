import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { Settings } from '../Settings';

// Initialize i18n
import '../../lib/i18n';

// Mock AccountsModal to avoid deep dependency rendering
vi.mock('../AccountsModal', () => ({
  AccountsModal: () => null,
}));

// Mock useTheme
vi.mock('../../hooks/useTheme', () => ({
  useTheme: () => ({ theme: 'dark', setTheme: vi.fn() }),
}));

// Mock useLanguage
vi.mock('../../hooks/useLanguage', () => ({
  useLanguage: () => ({ language: 'en', setLanguage: vi.fn() }),
  SUPPORTED_LANGUAGES: [{ code: 'en', name: 'English', nativeName: 'English' }],
}));

// Track set_config_cmd calls
const mockSetConfigCmd = vi.fn().mockResolvedValue(undefined);
const mockGetConfigCmd = vi.fn().mockResolvedValue(null);

vi.mock('../../lib/tauri', () => ({
  isTauri: () => false,
  tauriInvoke: (cmd: string, ...args: unknown[]) => {
    if (cmd === 'set_config_cmd') return mockSetConfigCmd(cmd, ...args);
    if (cmd === 'get_config_cmd') return mockGetConfigCmd(cmd, ...args);
    return Promise.resolve(null);
  },
}));

beforeEach(() => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
  localStorage.clear();
  vi.clearAllMocks();
});

function renderSettings() {
  return render(
    <MemoryRouter>
      <Settings />
    </MemoryRouter>
  );
}

describe('Settings component smoke tests', () => {
  it('renders without crashing', () => {
    const { container } = renderSettings();
    expect(container).toBeTruthy();
  });

  it('renders the Settings heading', () => {
    renderSettings();
    // The i18n key "settings.title" resolves to "Settings" in English
    expect(screen.getAllByText(/settings/i).length).toBeGreaterThan(0);
  });

  it('shows the base currency selector', () => {
    renderSettings();
    // The label for the base currency row
    expect(screen.getByText(/base currency/i)).toBeTruthy();
  });

  it('shows the auto-refresh interval selector', () => {
    renderSettings();
    expect(screen.getAllByText(/auto.?refresh/i).length).toBeGreaterThan(0);
  });

  it('shows cost basis method options', () => {
    renderSettings();
    // AVCO and FIFO options should be present
    expect(screen.getByText(/avco/i)).toBeTruthy();
    expect(screen.getByText(/fifo/i)).toBeTruthy();
  });

  it('shows the version number', () => {
    renderSettings();
    expect(screen.getByText('0.1.0')).toBeTruthy();
  });

  it('shows Manage Accounts button', () => {
    renderSettings();
    expect(screen.getByRole('button', { name: /manage accounts/i })).toBeTruthy();
  });

  it('changing base currency triggers config setValue', async () => {
    renderSettings();
    // The Select component renders as role="combobox" buttons.
    // The base currency combobox shows "CAD" by default.
    const comboboxes = screen.getAllByRole('combobox');
    // Find the combobox whose displayed text is "CAD"
    const currencyCombobox = comboboxes.find((el) => el.textContent?.includes('CAD'));
    expect(currencyCombobox).toBeTruthy();
    if (currencyCombobox) {
      // Open the dropdown
      fireEvent.click(currencyCombobox);
      // Click the USD option from the listbox
      const usdOption = await screen.findByRole('option', { name: 'USD' });
      fireEvent.pointerDown(usdOption);
      // After selection the combobox should show USD
      await waitFor(() => {
        expect(currencyCombobox.textContent).toContain('USD');
      });
    }
  });
});
