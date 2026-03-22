import { useState, useEffect } from 'react';
import i18next from '../lib/i18n';
import { isTauri, tauriInvoke } from '../lib/tauri';

export const SUPPORTED_LANGUAGES = [
  { code: 'en', name: 'English', nativeName: 'English' },
  { code: 'fr', name: 'French', nativeName: 'Français' },
  { code: 'de', name: 'German', nativeName: 'Deutsch' },
  { code: 'es', name: 'Spanish', nativeName: 'Español' },
  { code: 'pt', name: 'Portuguese', nativeName: 'Português' },
  { code: 'pl', name: 'Polish', nativeName: 'Polski' },
  { code: 'ja', name: 'Japanese', nativeName: '日本語' },
  { code: 'zh', name: 'Chinese', nativeName: '中文' },
];

export function useLanguage() {
  const [language, setLanguageState] = useState('en');

  useEffect(() => {
    const load = async () => {
      let saved: string | null = null;
      if (isTauri()) {
        try {
          saved = await tauriInvoke<string | null>('get_config_cmd', { key: 'app_language' });
        } catch {
          // Ignore errors; fall back to default language
        }
      } else {
        saved = localStorage.getItem('app_language');
      }
      if (saved && SUPPORTED_LANGUAGES.some((l) => l.code === saved)) {
        setLanguageState(saved);
        void i18next.changeLanguage(saved);
      }
    };
    void load();
  }, []);

  const setLanguage = async (code: string) => {
    setLanguageState(code);
    await i18next.changeLanguage(code);
    if (isTauri()) {
      await tauriInvoke('set_config_cmd', { key: 'app_language', value: code });
    } else {
      localStorage.setItem('app_language', code);
    }
  };

  return { language, setLanguage };
}
