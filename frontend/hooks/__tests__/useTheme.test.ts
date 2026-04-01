import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';

// Ensure non-Tauri browser path before each test
beforeEach(() => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
  localStorage.clear();
  document.documentElement.removeAttribute('data-theme');
});

afterEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute('data-theme');
  vi.restoreAllMocks();
});

describe('useTheme (non-Tauri path)', () => {
  it('defaults to dark theme when nothing is stored', async () => {
    const { useTheme } = await import('../../hooks/useTheme');
    const { result } = renderHook(() => useTheme());

    await waitFor(() => {
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    });

    expect(result.current.theme).toBe('dark');
  });

  it('applies stored theme from localStorage on mount', async () => {
    localStorage.setItem('app_theme', 'light');

    const { useTheme } = await import('../../hooks/useTheme');
    const { result } = renderHook(() => useTheme());

    await waitFor(() => {
      expect(result.current.theme).toBe('light');
    });

    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  });

  it('setTheme updates state, DOM attribute, and localStorage', async () => {
    const { useTheme } = await import('../../hooks/useTheme');
    const { result } = renderHook(() => useTheme());

    await waitFor(() => expect(result.current.theme).toBeDefined());

    await act(async () => {
      await result.current.setTheme('light');
    });

    expect(result.current.theme).toBe('light');
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
    expect(localStorage.getItem('app_theme')).toBe('light');
  });

  it('system theme resolves to dark when OS prefers dark', async () => {
    // Mock matchMedia to return prefers-color-scheme: dark
    vi.spyOn(window, 'matchMedia').mockImplementation((query) => ({
      matches: query === '(prefers-color-scheme: dark)',
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { useTheme } = await import('../../hooks/useTheme');
    const { result } = renderHook(() => useTheme());

    await act(async () => {
      await result.current.setTheme('system');
    });

    // When OS prefers dark, system mode should resolve to 'dark'
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });

  it('system theme resolves to light when OS prefers light', async () => {
    vi.spyOn(window, 'matchMedia').mockImplementation((query) => ({
      matches: false, // no dark preference
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { useTheme } = await import('../../hooks/useTheme');
    const { result } = renderHook(() => useTheme());

    await act(async () => {
      await result.current.setTheme('system');
    });

    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  });
});
