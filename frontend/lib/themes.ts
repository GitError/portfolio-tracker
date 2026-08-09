/**
 * Color scheme definitions for the Settings → Display selector.
 *
 * The actual colors are applied via CSS custom properties in `index.css`
 * (`[data-scheme="..."]` blocks) — the `colors` map here exists only to
 * render the small preview swatch in Settings, so it must be kept in sync
 * with the CSS values by hand.
 */

export type ColorSchemeKey = 'default' | 'dracula' | 'synthwave' | 'nord' | 'warmLight';

export interface ColorSchemeSwatch {
  bgPrimary: string;
  bgSurface: string;
  colorAccent: string;
  colorGain: string;
}

export interface ColorSchemeDefinition {
  key: ColorSchemeKey;
  nameKey: string;
  mode: 'dark' | 'light';
  swatch: ColorSchemeSwatch;
}

export const DEFAULT_COLOR_SCHEME: ColorSchemeKey = 'default';

export const COLOR_SCHEMES: ColorSchemeDefinition[] = [
  {
    key: 'default',
    nameKey: 'colorScheme.default',
    mode: 'dark',
    swatch: {
      bgPrimary: '#0a0a0f',
      bgSurface: '#12121a',
      colorAccent: '#3b82f6',
      colorGain: '#00d4aa',
    },
  },
  {
    key: 'dracula',
    nameKey: 'colorScheme.dracula',
    mode: 'dark',
    swatch: {
      bgPrimary: '#282a36',
      bgSurface: '#343746',
      colorAccent: '#ff92d0',
      colorGain: '#50fa7b',
    },
  },
  {
    key: 'synthwave',
    nameKey: 'colorScheme.synthwave',
    mode: 'dark',
    swatch: {
      bgPrimary: '#1a1033',
      bgSurface: '#241b42',
      colorAccent: '#ff5fb0',
      colorGain: '#39ff88',
    },
  },
  {
    key: 'nord',
    nameKey: 'colorScheme.nord',
    mode: 'dark',
    swatch: {
      bgPrimary: '#2e3440',
      bgSurface: '#3b4252',
      colorAccent: '#88c0d0',
      colorGain: '#a3be8c',
    },
  },
  {
    key: 'warmLight',
    nameKey: 'colorScheme.warmLight',
    mode: 'light',
    swatch: {
      bgPrimary: '#faf3e8',
      bgSurface: '#ffffff',
      colorAccent: '#a8531a',
      colorGain: '#2e7d4f',
    },
  },
];

export const COLOR_SCHEME_KEYS: ColorSchemeKey[] = COLOR_SCHEMES.map((s) => s.key);

export function isColorSchemeKey(value: string): value is ColorSchemeKey {
  return (COLOR_SCHEME_KEYS as string[]).includes(value);
}
