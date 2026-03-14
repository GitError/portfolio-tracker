import { useState, useEffect, useCallback } from 'react';

const isTauri = (): boolean => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

const LOCAL_STORAGE_KEY = 'app-config';

function readLocalConfig(key: string): string | null {
  try {
    const raw = localStorage.getItem(LOCAL_STORAGE_KEY);
    if (!raw) return null;
    return (JSON.parse(raw) as Record<string, string>)[key] ?? null;
  } catch {
    return null;
  }
}

function writeLocalConfig(key: string, value: string): void {
  try {
    const raw = localStorage.getItem(LOCAL_STORAGE_KEY);
    const config: Record<string, string> = raw ? (JSON.parse(raw) as Record<string, string>) : {};
    localStorage.setItem(LOCAL_STORAGE_KEY, JSON.stringify({ ...config, [key]: value }));
  } catch {
    // ignore
  }
}

export function useConfig(key: string, defaultValue: string) {
  const [value, setValue] = useState<string>(defaultValue);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        if (isTauri()) {
          const stored = await tauriInvoke<string | null>('get_config_cmd', { key });
          if (!cancelled) setValue(stored ?? defaultValue);
        } else {
          const stored = readLocalConfig(key);
          if (!cancelled) setValue(stored ?? defaultValue);
        }
      } catch {
        // keep default
      } finally {
        if (!cancelled) setReady(true);
      }
    }
    load();
    return () => {
      cancelled = true;
    };
  }, [key, defaultValue]);

  const persist = useCallback(
    async (newValue: string) => {
      setValue(newValue);
      try {
        if (isTauri()) {
          await tauriInvoke('set_config_cmd', { key, value: newValue });
        } else {
          writeLocalConfig(key, newValue);
        }
      } catch {
        // ignore persistence errors; state is still updated in-memory
      }
    },
    [key]
  );

  return { value, setValue: persist, ready };
}
