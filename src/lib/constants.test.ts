import { describe, it, expect } from 'vitest';
import { SUPPORTED_CURRENCIES, ASSET_TYPE_CONFIG, ACCOUNT_OPTIONS } from './constants';

describe('SUPPORTED_CURRENCIES', () => {
  it('contains no duplicate currency codes', () => {
    const seen = new Set<string>();
    for (const currency of SUPPORTED_CURRENCIES) {
      expect(seen.has(currency), `Duplicate currency found: ${currency}`).toBe(false);
      seen.add(currency);
    }
    expect(seen.size).toBe(SUPPORTED_CURRENCIES.length);
  });

  it('all currency codes are uppercase ISO 4217 format', () => {
    for (const currency of SUPPORTED_CURRENCIES) {
      expect(currency).toMatch(/^[A-Z]{3}$/);
    }
  });

  it('contains CAD and USD as primary trading currencies', () => {
    expect(SUPPORTED_CURRENCIES).toContain('CAD');
    expect(SUPPORTED_CURRENCIES).toContain('USD');
  });

  it('is non-empty', () => {
    expect(SUPPORTED_CURRENCIES.length).toBeGreaterThan(0);
  });
});

describe('ASSET_TYPE_CONFIG', () => {
  it('defines all four asset types', () => {
    expect(ASSET_TYPE_CONFIG).toHaveProperty('stock');
    expect(ASSET_TYPE_CONFIG).toHaveProperty('etf');
    expect(ASSET_TYPE_CONFIG).toHaveProperty('crypto');
    expect(ASSET_TYPE_CONFIG).toHaveProperty('cash');
  });

  it('each asset type has a label, color, and icon', () => {
    for (const [key, cfg] of Object.entries(ASSET_TYPE_CONFIG)) {
      expect(cfg.label, `${key}.label`).toBeTruthy();
      expect(cfg.color, `${key}.color`).toBeTruthy();
      expect(cfg.icon, `${key}.icon`).toBeTruthy();
    }
  });
});

describe('ACCOUNT_OPTIONS', () => {
  it('contains no duplicate account values', () => {
    const values = ACCOUNT_OPTIONS.map((o) => o.value);
    const unique = new Set(values);
    expect(unique.size).toBe(values.length);
  });

  it('each option has a non-empty label', () => {
    for (const option of ACCOUNT_OPTIONS) {
      expect(option.label.trim()).toBeTruthy();
    }
  });
});
