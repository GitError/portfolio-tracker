import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import i18next from '../../lib/i18n';

const EM_DASH = '—';

// Reset to English before each test to keep tests isolated.
beforeEach(async () => {
  await act(async () => {
    await i18next.changeLanguage('en');
  });
});

describe('useFormatters', () => {
  describe('formatCurrency', () => {
    it('returns em dash for NaN', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatCurrency(NaN)).toBe(EM_DASH);
    });

    it('returns em dash for Infinity', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatCurrency(Infinity)).toBe(EM_DASH);
    });

    it('returns em dash for null', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatCurrency(null as unknown as number)).toBe(EM_DASH);
    });

    it('returns em dash for undefined', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatCurrency(undefined as unknown as number)).toBe(EM_DASH);
    });

    it('formats zero with default CAD currency', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatCurrency(0)).toBe('0.00 CAD');
    });

    it('formats positive value with default CAD currency', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatCurrency(1234.56)).toBe('1,234.56 CAD');
    });

    it('formats negative value', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatCurrency(-500)).toBe('-500.00 CAD');
    });

    it('formats with custom currency label', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatCurrency(100, 'USD')).toBe('100.00 USD');
    });
  });

  describe('formatNumber', () => {
    it('returns em dash for NaN', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatNumber(NaN)).toBe(EM_DASH);
    });

    it('returns em dash for Infinity', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatNumber(Infinity)).toBe(EM_DASH);
    });

    it('formats with default 2 decimals', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatNumber(1234.5)).toBe('1,234.50');
    });

    it('formats with custom decimal places', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatNumber(1.5, 4)).toBe('1.5000');
    });

    it('formats zero', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatNumber(0)).toBe('0.00');
    });
  });

  describe('formatPercent', () => {
    it('returns em dash for NaN', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatPercent(NaN)).toBe(EM_DASH);
    });

    it('formats zero as +0.00%', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatPercent(0)).toBe('+0.00%');
    });

    it('formats positive value with leading plus sign', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatPercent(12.5)).toBe('+12.50%');
    });

    it('formats negative value without plus sign', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatPercent(-5.25)).toBe('-5.25%');
    });
  });

  describe('formatCompact', () => {
    it('returns em dash for NaN', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatCompact(NaN)).toBe(EM_DASH);
    });

    it('formats values under 1K with dollar sign', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatCompact(500)).toBe('$500.00');
    });

    it('formats values in thousands', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatCompact(1500)).toBe('$1.5K');
    });

    it('formats values in millions', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatCompact(2_500_000)).toBe('$2.5M');
    });

    it('formats negative values', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());
      expect(result.current.formatCompact(-3000)).toBe('-$3.0K');
    });
  });

  describe('language reactivity', () => {
    it('re-renders and uses new locale when language changes', async () => {
      const { useFormatters } = await import('../useFormatters');
      const { result } = renderHook(() => useFormatters());

      // In English, thousands separator is ','
      expect(result.current.formatCurrency(1234.56)).toBe('1,234.56 CAD');

      // Switch to German — thousands separator is '.' and decimal is ','
      await act(async () => {
        await i18next.changeLanguage('de');
      });

      // German locale formats 1234.56 as '1.234,56'
      expect(result.current.formatCurrency(1234.56)).toBe('1.234,56 CAD');

      // Restore English
      await act(async () => {
        await i18next.changeLanguage('en');
      });
    });
  });
});
