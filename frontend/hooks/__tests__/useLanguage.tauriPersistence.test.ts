import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';

const tauriInvokeMock = vi.fn();

// Isolated from useLanguage.test.ts (which exercises the non-Tauri/localStorage path) so we
// can force isTauri() to true and control tauriInvoke's resolution/rejection precisely.
vi.mock('../../lib/tauri', async () => {
  const actual = await vi.importActual<typeof import('../../lib/tauri')>('../../lib/tauri');
  return {
    ...actual,
    isTauri: () => true,
    tauriInvoke: (...args: unknown[]) =>
      (tauriInvokeMock as unknown as (...args: unknown[]) => unknown)(...args),
  };
});

beforeEach(() => {
  localStorage.clear();
  tauriInvokeMock.mockReset();
});

afterEach(() => {
  localStorage.clear();
});

describe('useLanguage (Tauri persistence path, #698)', () => {
  it('logs and surfaces a set_config_cmd persistence failure to the caller, without rolling back the already-applied language change', async () => {
    tauriInvokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_config_cmd') return null;
      if (cmd === 'set_config_cmd') throw new Error('disk full');
      return undefined;
    });
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const { useLanguage } = await import('../../hooks/useLanguage');
    const { result } = renderHook(() => useLanguage());
    await waitFor(() => expect(result.current.language).toBeDefined());

    let caught: unknown;
    await act(async () => {
      try {
        await result.current.setLanguage('fr');
      } catch (e) {
        caught = e;
      }
    });

    expect((caught as Error)?.message).toBe('disk full');

    // i18next.changeLanguage and the localStorage cache already succeeded before the
    // Tauri persistence call failed — only that last step failed, so state must not roll back.
    expect(result.current.language).toBe('fr');
    expect(localStorage.getItem('app_language')).toBe('fr');
    expect(consoleErrorSpy).toHaveBeenCalled();

    consoleErrorSpy.mockRestore();
  });

  it('persists the language via set_config_cmd when it resolves', async () => {
    tauriInvokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_config_cmd') return null;
      return undefined;
    });

    const { useLanguage } = await import('../../hooks/useLanguage');
    const { result } = renderHook(() => useLanguage());
    await waitFor(() => expect(result.current.language).toBeDefined());

    await act(async () => {
      await result.current.setLanguage('de');
    });

    expect(result.current.language).toBe('de');
    expect(tauriInvokeMock).toHaveBeenCalledWith('set_config_cmd', {
      key: 'app_language',
      value: 'de',
    });
  });
});
