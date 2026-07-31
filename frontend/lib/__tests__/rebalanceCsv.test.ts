import { describe, it, expect } from 'vitest';
import { buildCsvContent } from '../rebalanceCsv';
import type { RebalanceSuggestion } from '../../types/portfolio';

function makeSuggestion(overrides: Partial<RebalanceSuggestion> = {}): RebalanceSuggestion {
  return {
    holdingId: 'h1',
    symbol: 'AAPL',
    name: 'Apple Inc.',
    currentValueCad: 1000,
    targetValueCad: 1200,
    currentWeight: 10,
    targetWeight: 12,
    drift: 2,
    suggestedTradeCad: -200,
    suggestedUnits: 1.5,
    currentPriceCad: 150,
    ...overrides,
  };
}

describe('buildCsvContent', () => {
  it('does not alter ordinary symbol/name values', () => {
    const csv = buildCsvContent([makeSuggestion()]);
    const dataRow = csv.split('\n')[1] ?? '';
    expect(dataRow.startsWith('AAPL,"Apple Inc."')).toBe(true);
  });

  it.each([
    ['=', '=CMD(calc)'],
    ['+', '+1+1'],
    ['-', '-1+1'],
    ['@', '@SUM(A1)'],
    ['tab', '\tmalicious'],
    ['CR', '\rmalicious'],
  ])('neutralizes a %s-prefixed symbol', (_label, malicious) => {
    const csv = buildCsvContent([makeSuggestion({ symbol: malicious })]);
    const dataRow = csv.split('\n')[1] ?? '';
    expect(dataRow.startsWith(`'${malicious}`)).toBe(true);
  });

  it.each([
    ['=', '=CMD(calc)'],
    ['+', '+1+1'],
    ['-', '-1+1'],
    ['@', '@SUM(A1)'],
    ['tab', '\tmalicious'],
    ['CR', '\rmalicious'],
  ])('neutralizes a %s-prefixed name', (_label, malicious) => {
    const csv = buildCsvContent([makeSuggestion({ name: malicious })]);
    const dataRow = csv.split('\n')[1] ?? '';
    expect(dataRow).toContain(`"'${malicious}"`);
  });
});
