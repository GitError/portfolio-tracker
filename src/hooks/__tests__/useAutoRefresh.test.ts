import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { isWithinMarketHours } from '../useAutoRefresh';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Sets the system clock to a specific America/New_York wall-clock time.
 *
 * We use January 2024 (EST = UTC-5, no DST) so the offset is fixed and
 * predictable. The base Monday is 2024-01-15 00:00 UTC.
 *
 * weekday: 1=Mon 2=Tue 3=Wed 4=Thu 5=Fri 6=Sat 0=Sun
 */
function setEasternTime(weekday: number, hours: number, minutes: number): void {
  // Monday 2024-01-15 00:00 UTC
  const baseMondayUtcMs = Date.UTC(2024, 0, 15, 0, 0, 0, 0);
  const estOffsetMs = 5 * 60 * 60 * 1000; // UTC-5

  // Day offset relative to Monday (weekday 1)
  const dayOffsets: Record<number, number> = {
    1: 0, // Monday
    2: 1,
    3: 2,
    4: 3,
    5: 4, // Friday
    6: 5, // Saturday
    0: 6, // Sunday (following Sunday)
  };
  const dayOffset = dayOffsets[weekday] ?? 0;

  const targetUtcMs =
    baseMondayUtcMs +
    dayOffset * 24 * 60 * 60 * 1000 +
    estOffsetMs +
    hours * 60 * 60 * 1000 +
    minutes * 60 * 1000;

  vi.setSystemTime(new Date(targetUtcMs));
}

// ---------------------------------------------------------------------------
// isWithinMarketHours — pure function tests
// ---------------------------------------------------------------------------

describe('isWithinMarketHours', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns true on a weekday (Monday) at 10:00 ET', () => {
    setEasternTime(1, 10, 0);
    expect(isWithinMarketHours()).toBe(true);
  });

  it('returns true at market open 09:30 ET', () => {
    setEasternTime(2, 9, 30);
    expect(isWithinMarketHours()).toBe(true);
  });

  it('returns true just before close at 15:59 ET', () => {
    setEasternTime(3, 15, 59);
    expect(isWithinMarketHours()).toBe(true);
  });

  it('returns false at exactly 16:00 ET (market close)', () => {
    setEasternTime(4, 16, 0);
    expect(isWithinMarketHours()).toBe(false);
  });

  it('returns false on a weekday at 17:00 ET', () => {
    setEasternTime(5, 17, 0);
    expect(isWithinMarketHours()).toBe(false);
  });

  it('returns false before market open at 09:29 ET', () => {
    setEasternTime(1, 9, 29);
    expect(isWithinMarketHours()).toBe(false);
  });

  it('returns false on Saturday', () => {
    setEasternTime(6, 10, 0);
    expect(isWithinMarketHours()).toBe(false);
  });

  it('returns false on Sunday', () => {
    setEasternTime(0, 10, 0);
    expect(isWithinMarketHours()).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// useAutoRefresh hook — behaviour tests
// ---------------------------------------------------------------------------

// Mock useConfig so tests don't touch localStorage/Tauri.
// Each call gets an isolated in-memory store so different config keys work.
vi.mock('../useConfig', () => {
  const store: Record<string, string> = {};
  return {
    useConfig: vi.fn((key: string, defaultValue: string) => ({
      value: store[key] ?? defaultValue,
      setValue: vi.fn((v: string) => {
        store[key] = v;
      }),
      ready: true,
    })),
  };
});

describe('useAutoRefresh hook', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    Object.defineProperty(document, 'hidden', {
      value: false,
      configurable: true,
      writable: true,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('returns isEnabled=false and countdown=null when interval is 0 (default config)', async () => {
    const { useAutoRefresh } = await import('../useAutoRefresh');
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() => useAutoRefresh({ onRefresh }));

    expect(result.current.isEnabled).toBe(false);
    expect(result.current.countdown).toBeNull();
    expect(result.current.secondsUntilRefresh).toBe(0);
  });

  it('pauses countdown when document.hidden is true', async () => {
    const { useConfig } = await import('../useConfig');
    vi.mocked(useConfig).mockImplementation((key: string) => {
      if (key === 'auto_refresh_interval_ms') {
        return { value: '300000', setValue: vi.fn(), ready: true };
      }
      return { value: 'false', setValue: vi.fn(), ready: true };
    });

    const { useAutoRefresh } = await import('../useAutoRefresh');
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() => useAutoRefresh({ onRefresh }));

    const initialSeconds = result.current.secondsUntilRefresh;
    expect(initialSeconds).toBeGreaterThan(0);

    // Hide the document — countdown should reset and not fire
    Object.defineProperty(document, 'hidden', { value: true, configurable: true, writable: true });

    act(() => {
      vi.advanceTimersByTime(5000);
    });

    // When hidden, each tick resets to the full interval; onRefresh must not fire
    expect(onRefresh).not.toHaveBeenCalled();
    // Countdown resets back toward the full interval value
    expect(result.current.secondsUntilRefresh).toBeGreaterThanOrEqual(initialSeconds - 1);
  });

  it('does not call onRefresh when disabled (interval 0)', async () => {
    const { useConfig } = await import('../useConfig');
    vi.mocked(useConfig).mockImplementation((key: string) => {
      if (key === 'auto_refresh_interval_ms') {
        return { value: '0', setValue: vi.fn(), ready: true };
      }
      return { value: 'false', setValue: vi.fn(), ready: true };
    });

    const { useAutoRefresh } = await import('../useAutoRefresh');
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    renderHook(() => useAutoRefresh({ onRefresh }));

    act(() => {
      vi.advanceTimersByTime(600_000); // 10 minutes
    });

    expect(onRefresh).not.toHaveBeenCalled();
  });

  it('exposes setInterval, setMarketHoursOnly, and toggle functions', async () => {
    const { useAutoRefresh } = await import('../useAutoRefresh');
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() => useAutoRefresh({ onRefresh }));

    expect(typeof result.current.setInterval).toBe('function');
    expect(typeof result.current.setMarketHoursOnly).toBe('function');
    expect(typeof result.current.toggle).toBe('function');
  });

  it('exposes intervalMinutes and marketHoursOnly from config', async () => {
    const { useConfig } = await import('../useConfig');
    vi.mocked(useConfig).mockImplementation((key: string) => {
      if (key === 'auto_refresh_interval_ms') {
        return { value: '900000', setValue: vi.fn(), ready: true }; // 15 min
      }
      if (key === 'auto_refresh_market_hours_only') {
        return { value: 'true', setValue: vi.fn(), ready: true };
      }
      return { value: '', setValue: vi.fn(), ready: true };
    });

    const { useAutoRefresh } = await import('../useAutoRefresh');
    const { result } = renderHook(() => useAutoRefresh({ onRefresh: vi.fn().mockResolvedValue(undefined) }));

    expect(result.current.intervalMinutes).toBe(15);
    expect(result.current.marketHoursOnly).toBe(true);
    expect(result.current.isEnabled).toBe(true);
  });
});
