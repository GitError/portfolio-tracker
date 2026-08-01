import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';

beforeEach(() => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
  localStorage.clear();
});

afterEach(() => {
  localStorage.clear();
});

describe('useLanguage (non-Tauri path)', () => {
  it('defaults to English when nothing is stored', async () => {
    const { useLanguage } = await import('../../hooks/useLanguage');
    const { result } = renderHook(() => useLanguage());

    await waitFor(() => expect(result.current.language).toBe('en'));
  });

  it('loads stored language from localStorage on mount', async () => {
    localStorage.setItem('app_language', 'fr');

    const { useLanguage } = await import('../../hooks/useLanguage');
    const { result } = renderHook(() => useLanguage());

    await waitFor(() => expect(result.current.language).toBe('fr'));
  });

  it('setLanguage updates state and localStorage', async () => {
    const { useLanguage } = await import('../../hooks/useLanguage');
    const { result } = renderHook(() => useLanguage());

    await waitFor(() => expect(result.current.language).toBeDefined());

    await act(async () => {
      await result.current.setLanguage('de');
    });

    expect(result.current.language).toBe('de');
    expect(localStorage.getItem('app_language')).toBe('de');
  });

  it('ignores unsupported language codes from localStorage', async () => {
    localStorage.setItem('app_language', 'xx-INVALID');

    const { useLanguage } = await import('../../hooks/useLanguage');
    const { result } = renderHook(() => useLanguage());

    // Should stay at default 'en', not apply the invalid code
    await waitFor(() => expect(result.current.language).toBe('en'));
  });

  it('SUPPORTED_LANGUAGES contains the expected language codes', async () => {
    const { SUPPORTED_LANGUAGES } = await import('../../hooks/useLanguage');

    const codes = SUPPORTED_LANGUAGES.map((l) => l.code);
    expect(codes).toContain('en');
    expect(codes).toContain('fr');
    expect(codes).toContain('de');
    expect(codes).toContain('ja');
    expect(codes).toContain('zh');
  });

  it('each entry in SUPPORTED_LANGUAGES has code, name, and nativeName', async () => {
    const { SUPPORTED_LANGUAGES } = await import('../../hooks/useLanguage');

    for (const lang of SUPPORTED_LANGUAGES) {
      expect(lang.code).toBeTruthy();
      expect(lang.name).toBeTruthy();
      expect(lang.nativeName).toBeTruthy();
    }
  });

  it('setLanguage logs and rejects when i18next.changeLanguage fails, without persisting the change', async () => {
    const { useLanguage } = await import('../../hooks/useLanguage');
    const { default: i18next } = await import('../../lib/i18n');

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const changeLanguageSpy = vi
      .spyOn(i18next, 'changeLanguage')
      .mockRejectedValueOnce(new Error('locale bundle failed to load'));

    const { result } = renderHook(() => useLanguage());
    await waitFor(() => expect(result.current.language).toBeDefined());

    await expect(
      act(async () => {
        await result.current.setLanguage('de');
      })
    ).rejects.toThrow('locale bundle failed to load');

    expect(consoleErrorSpy).toHaveBeenCalled();
    // Guard against the pre-fix behavior (silent failure — localStorage still updated despite the rejection).
    expect(localStorage.getItem('app_language')).toBeNull();

    changeLanguageSpy.mockRestore();
    consoleErrorSpy.mockRestore();
  });
});
