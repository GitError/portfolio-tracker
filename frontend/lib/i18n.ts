import i18next from 'i18next';
import { initReactI18next } from 'react-i18next';

import en from '../locales/en/translation.json';
import fr from '../locales/fr/translation.json';
import de from '../locales/de/translation.json';
import es from '../locales/es/translation.json';
import pt from '../locales/pt/translation.json';
import ja from '../locales/ja/translation.json';
import zh from '../locales/zh/translation.json';

i18next.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    fr: { translation: fr },
    de: { translation: de },
    es: { translation: es },
    pt: { translation: pt },
    ja: { translation: ja },
    zh: { translation: zh },
  },
  lng: 'en',
  fallbackLng: 'en',
  interpolation: { escapeValue: false },
});

export default i18next;
