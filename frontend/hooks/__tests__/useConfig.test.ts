import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';

// Ensure isTauri() returns false
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

describe('useConfig hook (non-Tauri / localStorage path)', () => {
  it('returns the default value when nothing is stored', async () => {
    const { useConfig } = await import('../../hooks/useConfig');

    const { result } = renderHook(() => useConfig('test-key', 'default-val'));

    await waitFor(() => expect(result.current.ready).toBe(true));

    expect(result.current.value).toBe('default-val');
  });

  it('ready transitions to true after initial load', async () => {
    const { useConfig } = await import('../../hooks/useConfig');

    const { result } = renderHook(() => useConfig('another-key', 'some-default'));

    await waitFor(() => expect(result.current.ready).toBe(true));
  });

  it('persist() updates value in state and localStorage', async () => {
    const { useConfig } = await import('../../hooks/useConfig');

    const { result } = renderHook(() => useConfig('my-pref', 'old-value'));

    await waitFor(() => expect(result.current.ready).toBe(true));
    expect(result.current.value).toBe('old-value');

    await act(async () => {
      await result.current.setValue('new-value');
    });

    expect(result.current.value).toBe('new-value');

    // Also check localStorage was written
    const stored = localStorage.getItem('app-config');
    expect(stored).not.toBeNull();
    const parsed = JSON.parse(stored!) as Record<string, string>;
    expect(parsed['my-pref']).toBe('new-value');
  });

  it('reads a previously stored value from localStorage', async () => {
    // Pre-populate localStorage before the hook mounts
    localStorage.setItem('app-config', JSON.stringify({ 'pre-stored': 'cached-value' }));

    const { useConfig } = await import('../../hooks/useConfig');

    const { result } = renderHook(() => useConfig('pre-stored', 'fallback'));

    await waitFor(() => expect(result.current.ready).toBe(true));

    expect(result.current.value).toBe('cached-value');
  });

  it('uses default when localStorage key is absent', async () => {
    localStorage.setItem('app-config', JSON.stringify({ 'other-key': 'val' }));

    const { useConfig } = await import('../../hooks/useConfig');

    const { result } = renderHook(() => useConfig('missing-key', 'my-default'));

    await waitFor(() => expect(result.current.ready).toBe(true));

    expect(result.current.value).toBe('my-default');
  });

  it('different keys are independent', async () => {
    const { useConfig } = await import('../../hooks/useConfig');

    const { result: r1 } = renderHook(() => useConfig('key-a', 'alpha'));
    const { result: r2 } = renderHook(() => useConfig('key-b', 'beta'));

    await waitFor(() => expect(r1.current.ready).toBe(true));
    await waitFor(() => expect(r2.current.ready).toBe(true));

    await act(async () => {
      await r1.current.setValue('alpha-updated');
    });

    expect(r1.current.value).toBe('alpha-updated');
    expect(r2.current.value).toBe('beta');
  });
});
