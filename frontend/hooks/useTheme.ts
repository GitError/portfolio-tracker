import { useState, useEffect } from 'react';
import { isTauri, tauriInvoke } from '../lib/tauri';

export type ThemeMode = 'dark' | 'light' | 'system';

const CONFIG_KEY = 'app_theme';

function resolveTheme(mode: ThemeMode): 'dark' | 'light' {
  if (mode === 'system') {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }
  return mode;
}

function applyTheme(mode: ThemeMode): void {
  const resolved = resolveTheme(mode);
  document.documentElement.setAttribute('data-theme', resolved);
}

export function useTheme(): { theme: ThemeMode; setTheme: (mode: ThemeMode) => Promise<void> } {
  const [theme, setThemeState] = useState<ThemeMode>('dark');

  // Load persisted theme on mount and apply it immediately
  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        let saved: string | null = null;
        if (isTauri()) {
          saved = await tauriInvoke<string | null>('get_config_cmd', { key: CONFIG_KEY });
        } else {
          saved = localStorage.getItem(CONFIG_KEY);
        }
        const mode = (saved as ThemeMode) || 'dark';
        if (!cancelled) {
          setThemeState(mode);
          applyTheme(mode);
        }
      } catch {
        // keep default dark theme
        if (!cancelled) applyTheme('dark');
      }
    }
    load();
    return () => {
      cancelled = true;
    };
  }, []);

  // Listen for OS-level preference changes when in 'system' mode
  useEffect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = () => {
      if (theme === 'system') applyTheme('system');
    };
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, [theme]);

  const setTheme = async (mode: ThemeMode): Promise<void> => {
    setThemeState(mode);
    applyTheme(mode);
    try {
      if (isTauri()) {
        await tauriInvoke('set_config_cmd', { key: CONFIG_KEY, value: mode });
      } else {
        localStorage.setItem(CONFIG_KEY, mode);
      }
    } catch {
      // ignore persistence errors; theme is still applied in-memory
    }
  };

  return { theme, setTheme };
}
