import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ImportWizardModal } from '../ImportWizardModal';
import type {
  Account,
  ImportCommitResult,
  ImportPlan,
  NormalizedImportRow,
} from '../../../types/portfolio';

import i18next from '../../../lib/i18n';

let mockIsTauri = true;
const mockGetAccounts = vi.fn();
const mockParseImportFile = vi.fn();
const mockCommitImport = vi.fn();

vi.mock('../../../lib/tauri', () => ({
  isTauri: () => mockIsTauri,
  getErrorMessage: (e: unknown) => (e instanceof Error ? e.message : String(e)),
  tauriInvoke: (cmd: string, ...args: unknown[]) => {
    if (cmd === 'get_accounts') return mockGetAccounts(...args);
    if (cmd === 'parse_import_file') return mockParseImportFile(...args);
    if (cmd === 'commit_import') return mockCommitImport(...args);
    return Promise.resolve(null);
  },
}));

const ACCOUNT: Account = {
  id: 'acct-1',
  name: 'TD Taxable',
  accountType: 'taxable',
  institution: 'TD',
  createdAt: '2026-01-01T00:00:00Z',
};

function makeRow(overrides: Partial<NormalizedImportRow> = {}): NormalizedImportRow {
  return {
    rowNumber: 1,
    action: 'create',
    symbol: 'AAPL',
    resolvedSymbol: 'AAPL',
    name: 'Apple Inc.',
    assetType: 'stock',
    quantity: 10,
    costBasis: 150,
    costBasisSource: 'average_cost',
    currency: 'USD',
    bookValue: null,
    marketValue: null,
    exchange: 'NASDAQ',
    targetWeight: null,
    accountType: 'taxable',
    accountName: null,
    warnings: [],
    errors: [],
    raw: {},
    dividendYield: null,
    annualizedIncome: null,
    exDividendDate: null,
    ...overrides,
  };
}

function makePlan(overrides: Partial<ImportPlan> = {}): ImportPlan {
  return {
    profileDetected: 'GenericCSV',
    columnMappings: [
      { sourceHeader: 'Symbol', canonicalField: 'symbol', confidence: 'alias', reason: '' },
    ],
    rows: [makeRow()],
    countCreate: 1,
    countUpdate: 0,
    countSkip: 0,
    countNeedsFix: 0,
    countWarning: 0,
    suggestedAccountType: null,
    suggestedAccountNumber: null,
    cashRows: [],
    ...overrides,
  };
}

function csvFileWithPath(path: string, name = 'holdings.csv'): File {
  const file = new File(['symbol\nAAPL'], name, { type: 'text/csv' });
  Object.defineProperty(file, 'path', { value: path, configurable: true });
  return file;
}

const mockOnClose = vi.fn();
const mockOnImported = vi.fn();

function renderModal(props: Partial<React.ComponentProps<typeof ImportWizardModal>> = {}) {
  return render(
    <ImportWizardModal isOpen onClose={mockOnClose} onImported={mockOnImported} {...props} />
  );
}

async function selectAccount() {
  // findByRole retries until the accounts load and the Select renders, unlike
  // getByRole which can lose a race against the get_accounts promise under load.
  const combobox = await screen.findByRole('combobox');
  fireEvent.click(combobox);
  const option = await screen.findByText('TD Taxable (Taxable)');
  // Select's option list handles onPointerDown, not onClick (see ui/Select.tsx).
  fireEvent.pointerDown(option);
}

beforeEach(async () => {
  vi.clearAllMocks();
  mockIsTauri = true;
  mockGetAccounts.mockResolvedValue([ACCOUNT]);
  await i18next.changeLanguage('en');
});

describe('ImportWizardModal', () => {
  it('renders nothing when isOpen=false', () => {
    renderModal({ isOpen: false });
    expect(screen.queryByText('Import Plus Insights')).toBeNull();
  });

  it('shows the desktop-only notice in browser mode', () => {
    mockIsTauri = false;
    renderModal();
    expect(screen.getByText(/only available in the desktop app/i)).toBeTruthy();
  });

  it('loads accounts and shows the account picker when open', async () => {
    renderModal();
    await waitFor(() => expect(mockGetAccounts).toHaveBeenCalled());
    fireEvent.click(screen.getByRole('combobox'));
    expect(await screen.findByText('TD Taxable (Taxable)')).toBeTruthy();
  });

  it('parses the file and auto-advances to preview when every column is mapped', async () => {
    renderModal();
    await waitFor(() => expect(mockGetAccounts).toHaveBeenCalled());
    await selectAccount();

    mockParseImportFile.mockResolvedValue(makePlan());
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    fireEvent.change(input, { target: { files: [csvFileWithPath('/tmp/holdings.csv')] } });

    await waitFor(() =>
      expect(mockParseImportFile).toHaveBeenCalledWith({
        filePath: '/tmp/holdings.csv',
        context: {
          accountType: 'taxable',
          accountName: 'TD Taxable',
          accountId: 'acct-1',
          sourceProfile: null,
          columnOverrides: {},
        },
      })
    );

    expect(await screen.findByText('AAPL')).toBeTruthy();
    expect(screen.getByRole('button', { name: /import 1 holding/i })).toBeTruthy();
  });

  it('shows the column mapping step when a column is unrecognized, and continuing re-parses', async () => {
    renderModal();
    await waitFor(() => expect(mockGetAccounts).toHaveBeenCalled());
    await selectAccount();

    const planWithUnmapped = makePlan({
      columnMappings: [
        { sourceHeader: 'Symbol', canonicalField: 'symbol', confidence: 'alias', reason: '' },
        { sourceHeader: 'Weird Column', canonicalField: null, confidence: 'unmapped', reason: '' },
      ],
    });
    mockParseImportFile.mockResolvedValueOnce(planWithUnmapped);
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    fireEvent.change(input, { target: { files: [csvFileWithPath('/tmp/holdings.csv')] } });

    expect(await screen.findByText('Weird Column')).toBeTruthy();

    mockParseImportFile.mockResolvedValueOnce(makePlan());
    fireEvent.click(screen.getByRole('button', { name: /continue/i }));

    await waitFor(() => expect(mockParseImportFile).toHaveBeenCalledTimes(2));
    expect(await screen.findByText('AAPL')).toBeTruthy();
  });

  it('commits the plan and shows the result summary, notifying the caller', async () => {
    renderModal();
    await waitFor(() => expect(mockGetAccounts).toHaveBeenCalled());
    await selectAccount();

    mockParseImportFile.mockResolvedValue(makePlan());
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    fireEvent.change(input, { target: { files: [csvFileWithPath('/tmp/holdings.csv')] } });
    await screen.findByText('AAPL');

    const commitResult: ImportCommitResult = {
      created: 1,
      updated: 0,
      skipped: 0,
      errors: [],
      newSymbols: ['AAPL'],
      changedSymbols: [],
      missingFromImport: [],
      staleSymbols: [],
    };
    mockCommitImport.mockResolvedValue(commitResult);

    fireEvent.click(screen.getByRole('button', { name: /import 1 holding/i }));

    await waitFor(() =>
      expect(mockCommitImport).toHaveBeenCalledWith({
        request: {
          planRows: [makePlan().rows[0]],
          accountId: 'acct-1',
          includeCash: false,
        },
      })
    );
    expect(await screen.findByText('Created 1, updated 0')).toBeTruthy();
    expect(mockOnImported).toHaveBeenCalled();
  });

  it('shows a commit error with a way back to preview', async () => {
    renderModal();
    await waitFor(() => expect(mockGetAccounts).toHaveBeenCalled());
    await selectAccount();

    mockParseImportFile.mockResolvedValue(makePlan());
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    fireEvent.change(input, { target: { files: [csvFileWithPath('/tmp/holdings.csv')] } });
    await screen.findByText('AAPL');

    mockCommitImport.mockRejectedValue(new Error('Database is locked'));
    fireEvent.click(screen.getByRole('button', { name: /import 1 holding/i }));

    expect(await screen.findByText('Database is locked')).toBeTruthy();
    expect(mockOnImported).not.toHaveBeenCalled();
  });

  it('rejects a non-csv file client-side without calling parse_import_file', async () => {
    renderModal();
    await waitFor(() => expect(mockGetAccounts).toHaveBeenCalled());
    await selectAccount();

    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    const badFile = new File(['x'], 'holdings.xlsx', { type: 'application/vnd.ms-excel' });
    Object.defineProperty(badFile, 'path', { value: '/tmp/holdings.xlsx', configurable: true });
    fireEvent.change(input, { target: { files: [badFile] } });

    expect(await screen.findByText(/only \.csv files are supported/i)).toBeTruthy();
    expect(mockParseImportFile).not.toHaveBeenCalled();
  });
});
