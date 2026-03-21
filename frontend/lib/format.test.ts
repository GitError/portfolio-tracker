import { describe, it, expect } from 'vitest';
import { formatCurrency, formatPercent, formatNumber, formatCompact } from './format';

describe('formatCurrency', () => {
  it('formats a positive number', () => {
    expect(formatCurrency(1234.56, 'CAD')).toBe('1,234.56 CAD');
  });

  it('formats zero', () => {
    expect(formatCurrency(0, 'CAD')).toBe('0.00 CAD');
  });

  it('formats a negative number', () => {
    const result = formatCurrency(-500.5, 'USD');
    expect(result).toContain('-500.50');
    expect(result).toContain('USD');
  });

  it('returns placeholder for NaN without throwing', () => {
    expect(() => formatCurrency(NaN)).not.toThrow();
    expect(formatCurrency(NaN)).toBe('—');
  });

  it('returns placeholder for Infinity without throwing', () => {
    expect(() => formatCurrency(Infinity)).not.toThrow();
    expect(formatCurrency(Infinity)).toBe('—');
  });

  it('returns placeholder for negative Infinity without throwing', () => {
    expect(() => formatCurrency(-Infinity)).not.toThrow();
    expect(formatCurrency(-Infinity)).toBe('—');
  });

  it('returns placeholder for undefined without throwing', () => {
    expect(() => formatCurrency(undefined)).not.toThrow();
    expect(formatCurrency(undefined)).toBe('—');
  });

  it('returns placeholder for null without throwing', () => {
    expect(() => formatCurrency(null)).not.toThrow();
    expect(formatCurrency(null)).toBe('—');
  });

  it('defaults to CAD when no currency is provided', () => {
    expect(formatCurrency(100)).toContain('CAD');
  });
});

describe('formatPercent', () => {
  it('formats zero', () => {
    expect(formatPercent(0)).toBe('+0.00%');
  });

  it('formats a negative decimal', () => {
    expect(formatPercent(-12.34)).toBe('-12.34%');
  });

  it('formats a positive decimal', () => {
    expect(formatPercent(5.5)).toBe('+5.50%');
  });

  it('formats -0.1234 as -0.12%', () => {
    // -0.1234 is treated as a raw percent value (already in % units), not a decimal fraction
    expect(formatPercent(-0.1234)).toBe('-0.12%');
  });

  it('returns placeholder for NaN without throwing', () => {
    expect(() => formatPercent(NaN)).not.toThrow();
    expect(formatPercent(NaN)).toBe('—');
  });

  it('returns placeholder for Infinity without throwing', () => {
    expect(() => formatPercent(Infinity)).not.toThrow();
    expect(formatPercent(Infinity)).toBe('—');
  });

  it('returns placeholder for undefined without throwing', () => {
    expect(() => formatPercent(undefined)).not.toThrow();
    expect(formatPercent(undefined)).toBe('—');
  });

  it('returns placeholder for null without throwing', () => {
    expect(() => formatPercent(null)).not.toThrow();
    expect(formatPercent(null)).toBe('—');
  });
});

describe('formatNumber', () => {
  it('formats with default 2 decimal places', () => {
    expect(formatNumber(1234.567)).toBe('1,234.57');
  });

  it('formats with custom decimal places', () => {
    expect(formatNumber(1234.5678, 4)).toBe('1,234.5678');
  });

  it('returns placeholder for NaN', () => {
    expect(formatNumber(NaN)).toBe('—');
  });

  it('returns placeholder for Infinity', () => {
    expect(formatNumber(Infinity)).toBe('—');
  });

  it('returns placeholder for undefined', () => {
    expect(formatNumber(undefined)).toBe('—');
  });

  it('returns placeholder for null', () => {
    expect(formatNumber(null)).toBe('—');
  });
});

describe('formatCompact', () => {
  it('abbreviates millions', () => {
    expect(formatCompact(1_500_000)).toContain('M');
  });

  it('abbreviates thousands', () => {
    expect(formatCompact(52_300)).toContain('K');
  });

  it('formats small values with $ prefix', () => {
    const result = formatCompact(999);
    expect(result).toContain('$');
    expect(result).not.toContain('K');
    expect(result).not.toContain('M');
  });

  it('handles negative millions', () => {
    const result = formatCompact(-2_000_000);
    expect(result).toContain('-');
    expect(result).toContain('M');
  });

  it('returns placeholder for NaN', () => {
    expect(formatCompact(NaN)).toBe('—');
  });

  it('returns placeholder for Infinity', () => {
    expect(formatCompact(Infinity)).toBe('—');
  });

  it('returns placeholder for undefined', () => {
    expect(formatCompact(undefined)).toBe('—');
  });

  it('returns placeholder for null', () => {
    expect(formatCompact(null)).toBe('—');
  });
});
