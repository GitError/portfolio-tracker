import { describe, it, expect } from 'vitest';
import { createPresetScenarioInfo, fxShockKey } from '../constants';

const HISTORICAL_NAMES = [
  '2008 Global Financial Crisis',
  'COVID-19 Crash',
  '2022 Rate-Hike Cycle',
  'Dot-Com Crash',
  'Black Monday 1987',
];

describe('historical scenario presets', () => {
  it('includes all named historical events, each marked isHistorical with a dataSource', () => {
    const presets = createPresetScenarioInfo('CAD');
    for (const name of HISTORICAL_NAMES) {
      const preset = presets.find((p) => p.name === name);
      expect(preset, `missing preset: ${name}`).toBeTruthy();
      expect(preset!.isHistorical).toBe(true);
      expect(preset!.dataSource).toBeTruthy();
      expect(preset!.dataSource!.length).toBeGreaterThan(0);
    }
  });

  it('does not mark existing hypothetical presets as historical', () => {
    const presets = createPresetScenarioInfo('CAD');
    const bearMarket = presets.find((p) => p.name === 'Bear Market');
    expect(bearMarket).toBeTruthy();
    expect(bearMarket!.isHistorical).toBeUndefined();
    expect(bearMarket!.dataSource).toBeUndefined();
  });

  it('gives every historical preset at least one non-zero shock', () => {
    const presets = createPresetScenarioInfo('CAD');
    for (const name of HISTORICAL_NAMES) {
      const preset = presets.find((p) => p.name === name)!;
      const values = Object.values(preset.shocks);
      expect(values.length).toBeGreaterThan(0);
      expect(values.some((v) => v !== 0)).toBe(true);
    }
  });

  it('omits crypto shocks for events that predate Bitcoin', () => {
    const presets = createPresetScenarioInfo('CAD');
    const gfc = presets.find((p) => p.name === '2008 Global Financial Crisis')!;
    const dotcom = presets.find((p) => p.name === 'Dot-Com Crash')!;
    const blackMonday = presets.find((p) => p.name === 'Black Monday 1987')!;
    expect(gfc.shocks.crypto).toBeUndefined();
    expect(dotcom.shocks.crypto).toBeUndefined();
    expect(blackMonday.shocks.crypto).toBeUndefined();
  });

  it('includes a crypto shock for post-2009 historical events', () => {
    const presets = createPresetScenarioInfo('CAD');
    const covid = presets.find((p) => p.name === 'COVID-19 Crash')!;
    const rateHike = presets.find((p) => p.name === '2022 Rate-Hike Cycle')!;
    expect(covid.shocks.crypto).toBeLessThan(0);
    expect(rateHike.shocks.crypto).toBeLessThan(0);
  });

  it('applies FX shocks under the base-currency-specific key', () => {
    const presets = createPresetScenarioInfo('CAD');
    const gfc = presets.find((p) => p.name === '2008 Global Financial Crisis')!;
    expect(gfc.shocks[fxShockKey('USD', 'CAD')]).toBeGreaterThan(0);
  });

  it('omits the USD/CAD shock entirely when the base currency is USD', () => {
    const presets = createPresetScenarioInfo('USD');
    const gfc = presets.find((p) => p.name === '2008 Global Financial Crisis')!;
    expect(Object.keys(gfc.shocks)).not.toContain(fxShockKey('USD', 'USD'));
  });

  it('keeps every shock value within the [-1, 1] range', () => {
    const presets = createPresetScenarioInfo('CAD');
    for (const name of HISTORICAL_NAMES) {
      const preset = presets.find((p) => p.name === name)!;
      for (const value of Object.values(preset.shocks)) {
        expect(value).toBeGreaterThanOrEqual(-1);
        expect(value).toBeLessThanOrEqual(1);
      }
    }
  });
});
