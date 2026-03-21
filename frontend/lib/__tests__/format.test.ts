import { describe, it, expect } from 'vitest';
import { formatCurrency, formatPercent, formatNumber, formatCompact } from '../format';

const EM_DASH = '—';

describe('formatCurrency', () => {
  describe('invalid inputs return em dash', () => {
    it('handles NaN', () => {
      expect(formatCurrency(NaN)).toBe(EM_DASH);
    });

    it('handles Infinity', () => {
      expect(formatCurrency(Infinity)).toBe(EM_DASH);
    });

    it('handles -Infinity', () => {
      expect(formatCurrency(-Infinity)).toBe(EM_DASH);
    });

    it('handles undefined cast as any', () => {
      expect(formatCurrency(undefined as unknown as number)).toBe(EM_DASH);
    });

    it('handles null cast as any', () => {
      expect(formatCurrency(null as unknown as number)).toBe(EM_DASH);
    });
  });

  describe('valid inputs format correctly', () => {
    it('formats zero', () => {
      expect(formatCurrency(0)).toBe('0.00 CAD');
    });

    it('formats a positive value with default CAD currency', () => {
      expect(formatCurrency(1234.56)).toBe('1,234.56 CAD');
    });

    it('formats a negative value', () => {
      expect(formatCurrency(-500)).toBe('-500.00 CAD');
    });

    it('formats with custom currency label', () => {
      expect(formatCurrency(100, 'USD')).toBe('100.00 USD');
    });
  });
});

describe('formatPercent', () => {
  describe('invalid inputs return em dash', () => {
    it('handles NaN', () => {
      expect(formatPercent(NaN)).toBe(EM_DASH);
    });

    it('handles Infinity', () => {
      expect(formatPercent(Infinity)).toBe(EM_DASH);
    });

    it('handles -Infinity', () => {
      expect(formatPercent(-Infinity)).toBe(EM_DASH);
    });

    it('handles undefined cast as any', () => {
      expect(formatPercent(undefined as unknown as number)).toBe(EM_DASH);
    });

    it('handles null cast as any', () => {
      expect(formatPercent(null as unknown as number)).toBe(EM_DASH);
    });
  });

  describe('valid inputs format correctly', () => {
    it('formats zero as +0.00%', () => {
      expect(formatPercent(0)).toBe('+0.00%');
    });

    it('formats a positive value with leading plus sign', () => {
      expect(formatPercent(12.5)).toBe('+12.50%');
    });

    it('formats a negative value without plus sign', () => {
      expect(formatPercent(-5.25)).toBe('-5.25%');
    });
  });
});

describe('formatNumber', () => {
  describe('invalid inputs return em dash', () => {
    it('handles NaN', () => {
      expect(formatNumber(NaN)).toBe(EM_DASH);
    });

    it('handles Infinity', () => {
      expect(formatNumber(Infinity)).toBe(EM_DASH);
    });

    it('handles -Infinity', () => {
      expect(formatNumber(-Infinity)).toBe(EM_DASH);
    });

    it('handles undefined cast as any', () => {
      expect(formatNumber(undefined as unknown as number)).toBe(EM_DASH);
    });
  });

  describe('valid inputs format correctly', () => {
    it('formats with default 2 decimals', () => {
      expect(formatNumber(1234.5)).toBe('1,234.50');
    });

    it('formats with custom decimal places', () => {
      expect(formatNumber(1.5, 4)).toBe('1.5000');
    });

    it('formats zero', () => {
      expect(formatNumber(0)).toBe('0.00');
    });
  });
});

describe('formatCompact', () => {
  describe('invalid inputs return em dash', () => {
    it('handles NaN', () => {
      expect(formatCompact(NaN)).toBe(EM_DASH);
    });

    it('handles Infinity', () => {
      expect(formatCompact(Infinity)).toBe(EM_DASH);
    });

    it('handles -Infinity', () => {
      expect(formatCompact(-Infinity)).toBe(EM_DASH);
    });

    it('handles undefined cast as any', () => {
      expect(formatCompact(undefined as unknown as number)).toBe(EM_DASH);
    });
  });

  describe('valid inputs format correctly', () => {
    it('formats values under 1K with dollar sign', () => {
      expect(formatCompact(500)).toBe('$500.00');
    });

    it('formats values in thousands', () => {
      expect(formatCompact(1500)).toBe('$1.5K');
    });

    it('formats values in millions', () => {
      expect(formatCompact(2_500_000)).toBe('$2.5M');
    });

    it('formats negative values in thousands', () => {
      expect(formatCompact(-3000)).toBe('-$3.0K');
    });

    it('formats zero', () => {
      expect(formatCompact(0)).toBe('$0.00');
    });
  });
});
