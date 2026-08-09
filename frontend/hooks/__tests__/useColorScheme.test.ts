import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';

beforeEach(() => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI_INTERNALS__;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  delete (window as any).__TAURI__;
  localStorage.clear();
  document.documentElement.removeAttribute('data-scheme');
});

afterEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute('data-scheme');
  vi.restoreAllMocks();
});

describe('useColorScheme (non-Tauri path)', () => {
  it('defaults to the default scheme with no data-scheme attribute when nothing is stored', async () => {
    const { useColorScheme } = await import('../../hooks/useColorScheme');
    const { result } = renderHook(() => useColorScheme());

    await waitFor(() => {
      expect(result.current.colorScheme).toBe('default');
    });

    expect(document.documentElement.hasAttribute('data-scheme')).toBe(false);
  });

  it('applies a stored scheme from localStorage on mount', async () => {
    localStorage.setItem('app_color_scheme', 'dracula');

    const { useColorScheme } = await import('../../hooks/useColorScheme');
    const { result } = renderHook(() => useColorScheme());

    await waitFor(() => {
      expect(result.current.colorScheme).toBe('dracula');
    });

    expect(document.documentElement.getAttribute('data-scheme')).toBe('dracula');
  });

  it('falls back to default when the stored value is not a known scheme', async () => {
    localStorage.setItem('app_color_scheme', 'solarized');

    const { useColorScheme } = await import('../../hooks/useColorScheme');
    const { result } = renderHook(() => useColorScheme());

    await waitFor(() => {
      expect(result.current.colorScheme).toBe('default');
    });

    expect(document.documentElement.hasAttribute('data-scheme')).toBe(false);
  });

  it('setColorScheme updates state, DOM attribute, and localStorage', async () => {
    const { useColorScheme } = await import('../../hooks/useColorScheme');
    const { result } = renderHook(() => useColorScheme());

    await waitFor(() => expect(result.current.colorScheme).toBeDefined());

    await act(async () => {
      await result.current.setColorScheme('synthwave');
    });

    expect(result.current.colorScheme).toBe('synthwave');
    expect(document.documentElement.getAttribute('data-scheme')).toBe('synthwave');
    expect(localStorage.getItem('app_color_scheme')).toBe('synthwave');
  });

  it('setColorScheme back to default removes the data-scheme attribute', async () => {
    const { useColorScheme } = await import('../../hooks/useColorScheme');
    const { result } = renderHook(() => useColorScheme());

    await act(async () => {
      await result.current.setColorScheme('nord');
    });
    expect(document.documentElement.getAttribute('data-scheme')).toBe('nord');

    await act(async () => {
      await result.current.setColorScheme('default');
    });

    expect(result.current.colorScheme).toBe('default');
    expect(document.documentElement.hasAttribute('data-scheme')).toBe(false);
    expect(localStorage.getItem('app_color_scheme')).toBe('default');
  });
});
