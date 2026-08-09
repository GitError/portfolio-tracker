import { useEffect, useState } from 'react';
import { isTauri, tauriInvoke } from '../lib/tauri';
import { DEFAULT_COLOR_SCHEME, isColorSchemeKey, type ColorSchemeKey } from '../lib/themes';

const CONFIG_KEY = 'app_color_scheme';

function applyColorScheme(scheme: ColorSchemeKey): void {
  const root = document.documentElement;
  if (scheme === DEFAULT_COLOR_SCHEME) {
    root.removeAttribute('data-scheme');
  } else {
    root.setAttribute('data-scheme', scheme);
  }
}

export function useColorScheme(): {
  colorScheme: ColorSchemeKey;
  setColorScheme: (scheme: ColorSchemeKey) => Promise<void>;
} {
  const [colorScheme, setColorSchemeState] = useState<ColorSchemeKey>(DEFAULT_COLOR_SCHEME);

  // Load persisted scheme on mount and apply it immediately
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
        const scheme = saved && isColorSchemeKey(saved) ? saved : DEFAULT_COLOR_SCHEME;
        if (!cancelled) {
          setColorSchemeState(scheme);
          applyColorScheme(scheme);
        }
      } catch {
        // keep default scheme
        if (!cancelled) applyColorScheme(DEFAULT_COLOR_SCHEME);
      }
    }
    load();
    return () => {
      cancelled = true;
    };
  }, []);

  const setColorScheme = async (scheme: ColorSchemeKey): Promise<void> => {
    setColorSchemeState(scheme);
    applyColorScheme(scheme);
    try {
      // Mirror to localStorage even in Tauri mode: index.html's pre-mount
      // script only has synchronous access to localStorage, not the async
      // Tauri config store, so this cache is what prevents a flash of the
      // wrong scheme on the next app launch.
      localStorage.setItem(CONFIG_KEY, scheme);
      if (isTauri()) {
        await tauriInvoke('set_config_cmd', { key: CONFIG_KEY, value: scheme });
      }
    } catch {
      // ignore persistence errors; scheme is still applied in-memory
    }
  };

  return { colorScheme, setColorScheme };
}
