import { describe, it, expect } from 'vitest';
import type { ColumnMapping, ImportPlan, NormalizedImportRow } from '../../types/portfolio';
import {
  buildCommitRequest,
  computeStatusCounts,
  countCommittable,
  filterRowsByStatus,
  needsManualMapping,
  rowFixHint,
} from '../importWizard';

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

function makeMapping(overrides: Partial<ColumnMapping> = {}): ColumnMapping {
  return {
    sourceHeader: 'Symbol',
    canonicalField: 'symbol',
    confidence: 'alias',
    reason: '',
    ...overrides,
  };
}

function makePlan(overrides: Partial<ImportPlan> = {}): ImportPlan {
  return {
    profileDetected: 'GenericCSV',
    columnMappings: [],
    rows: [],
    countCreate: 0,
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

describe('computeStatusCounts', () => {
  it('tallies rows by action', () => {
    const rows = [
      makeRow({ action: 'create' }),
      makeRow({ action: 'create' }),
      makeRow({ action: 'update' }),
      makeRow({ action: 'skip' }),
      makeRow({ action: 'needs_fix' }),
      makeRow({ action: 'warning' }),
    ];
    expect(computeStatusCounts(rows)).toEqual({
      create: 2,
      update: 1,
      skip: 1,
      needs_fix: 1,
      warning: 1,
    });
  });

  it('returns all-zero counts for an empty row list', () => {
    expect(computeStatusCounts([])).toEqual({
      create: 0,
      update: 0,
      skip: 0,
      needs_fix: 0,
      warning: 0,
    });
  });
});

describe('filterRowsByStatus', () => {
  const rows = [
    makeRow({ rowNumber: 1, action: 'create' }),
    makeRow({ rowNumber: 2, action: 'skip' }),
    makeRow({ rowNumber: 3, action: 'create' }),
  ];

  it('returns every row when filter is "all"', () => {
    expect(filterRowsByStatus(rows, 'all')).toHaveLength(3);
  });

  it('returns only rows matching the given action', () => {
    const result = filterRowsByStatus(rows, 'create');
    expect(result.map((r) => r.rowNumber)).toEqual([1, 3]);
  });

  it('returns an empty array when no row matches', () => {
    expect(filterRowsByStatus(rows, 'warning')).toEqual([]);
  });
});

describe('needsManualMapping', () => {
  it('is false when every mapping resolved to a canonical field', () => {
    const mappings = [
      makeMapping({ canonicalField: 'symbol' }),
      makeMapping({ canonicalField: 'quantity' }),
    ];
    expect(needsManualMapping(mappings)).toBe(false);
  });

  it('is true when at least one mapping is unmapped', () => {
    const mappings = [
      makeMapping({ canonicalField: 'symbol' }),
      makeMapping({ canonicalField: null, confidence: 'unmapped' }),
    ];
    expect(needsManualMapping(mappings)).toBe(true);
  });

  it('is false for an empty mapping list', () => {
    expect(needsManualMapping([])).toBe(false);
  });
});

describe('countCommittable', () => {
  it('counts create, update, and warning rows but not skip or needs_fix', () => {
    const rows = [
      makeRow({ action: 'create' }),
      makeRow({ action: 'update' }),
      makeRow({ action: 'warning' }),
      makeRow({ action: 'skip' }),
      makeRow({ action: 'needs_fix' }),
    ];
    expect(countCommittable(rows)).toBe(3);
  });
});

describe('rowFixHint', () => {
  it('joins error messages for a needs_fix row', () => {
    const row = makeRow({ action: 'needs_fix', errors: ['Missing currency', 'Quantity is zero'] });
    expect(rowFixHint(row)).toBe('Missing currency; Quantity is zero');
  });

  it('joins warning messages for a warning row', () => {
    const row = makeRow({ action: 'warning', warnings: ['Settlement currency mismatch'] });
    expect(rowFixHint(row)).toBe('Settlement currency mismatch');
  });

  it('returns null for a row with no errors or warnings', () => {
    const row = makeRow({ action: 'create' });
    expect(rowFixHint(row)).toBeNull();
  });
});

describe('buildCommitRequest', () => {
  it('combines rows and cashRows into planRows and passes through accountId/includeCash', () => {
    const plan = makePlan({
      rows: [makeRow({ rowNumber: 1 })],
      cashRows: [makeRow({ rowNumber: 2, assetType: 'cash' })],
    });
    const request = buildCommitRequest(plan, 'acct-1', true);
    expect(request.accountId).toBe('acct-1');
    expect(request.includeCash).toBe(true);
    expect(request.planRows.map((r) => r.rowNumber)).toEqual([1, 2]);
  });

  it('still includes cash rows in planRows when includeCash is false (backend filters them)', () => {
    const plan = makePlan({
      rows: [makeRow({ rowNumber: 1 })],
      cashRows: [makeRow({ rowNumber: 2, assetType: 'cash' })],
    });
    const request = buildCommitRequest(plan, 'acct-1', false);
    expect(request.includeCash).toBe(false);
    expect(request.planRows.map((r) => r.rowNumber)).toEqual([1, 2]);
  });
});
