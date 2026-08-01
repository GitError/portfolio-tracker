import { describe, expect, it } from 'vitest';

import de from '../../locales/de/translation.json';
import en from '../../locales/en/translation.json';
import es from '../../locales/es/translation.json';
import fr from '../../locales/fr/translation.json';
import ja from '../../locales/ja/translation.json';
import pl from '../../locales/pl/translation.json';
import pt from '../../locales/pt/translation.json';
import zh from '../../locales/zh/translation.json';

type TranslationTree = { [key: string]: string | TranslationTree };

function flattenKeys(tree: TranslationTree, prefix = ''): Set<string> {
  const keys = new Set<string>();
  for (const [key, value] of Object.entries(tree)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (typeof value === 'object' && value !== null) {
      for (const nested of flattenKeys(value, fullKey)) keys.add(nested);
    } else {
      keys.add(fullKey);
    }
  }
  return keys;
}

const LOCALES: Record<string, TranslationTree> = { de, es, fr, ja, pl, pt, zh };

describe('locale key parity', () => {
  const enKeys = flattenKeys(en);

  it.each(Object.entries(LOCALES))('%s has every English key', (_locale, tree) => {
    const keys = flattenKeys(tree);
    const missing = [...enKeys].filter((k) => !keys.has(k));
    expect(missing).toEqual([]);
  });
});
