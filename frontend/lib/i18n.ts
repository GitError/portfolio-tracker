import i18next from 'i18next';
import { initReactI18next } from 'react-i18next';

import en from '../locales/en/translation.json';
import fr from '../locales/fr/translation.json';
import de from '../locales/de/translation.json';
import es from '../locales/es/translation.json';
import pt from '../locales/pt/translation.json';
import pl from '../locales/pl/translation.json';
import ja from '../locales/ja/translation.json';
import zh from '../locales/zh/translation.json';

const SUPPORTED_LNG_CODES = ['en', 'fr', 'de', 'es', 'pt', 'pl', 'ja', 'zh'];

function detectInitialLanguage(): string {
  try {
    const saved = localStorage.getItem('app_language');
    if (saved && SUPPORTED_LNG_CODES.includes(saved)) return saved;
  } catch {
    // localStorage unavailable (e.g. SSR or sandboxed context)
  }
  return 'en';
}

i18next.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    fr: { translation: fr },
    de: { translation: de },
    es: { translation: es },
    pt: { translation: pt },
    pl: { translation: pl },
    ja: { translation: ja },
    zh: { translation: zh },
  },
  lng: detectInitialLanguage(),
  fallbackLng: 'en',
  interpolation: { escapeValue: false },
});

export default i18next;
