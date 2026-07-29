import { describe, it, expect, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import i18next from '../../lib/i18n';

afterEach(async () => {
  await act(async () => {
    await i18next.changeLanguage('en');
  });
});

describe('useFormatCurrency', () => {
  it('reflects the active language immediately after it changes', async () => {
    const { useFormatCurrency } = await import('../useFormatters');
    const { result } = renderHook(() => useFormatCurrency());

    expect(result.current(1234.56, 'USD')).toBe('1,234.56 USD');

    await act(async () => {
      await i18next.changeLanguage('de');
    });

    await waitFor(() => expect(result.current(1234.56, 'USD')).toBe('1.234,56 USD'));
  });
});

describe('useFormatNumber', () => {
  it('reflects the active language immediately after it changes', async () => {
    const { useFormatNumber } = await import('../useFormatters');
    const { result } = renderHook(() => useFormatNumber());

    expect(result.current(1234.5)).toBe('1,234.50');

    await act(async () => {
      await i18next.changeLanguage('de');
    });

    await waitFor(() => expect(result.current(1234.5)).toBe('1.234,50'));
  });
});
