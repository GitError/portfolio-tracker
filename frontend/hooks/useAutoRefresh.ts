import { useCallback, useEffect, useRef, useState } from 'react';
import { useConfig } from './useConfig';

// Config key constants
const CONFIG_KEY_INTERVAL = 'auto_refresh_interval_ms';
const CONFIG_KEY_MARKET_HOURS = 'auto_refresh_market_hours_only';

// Valid interval values in ms (0 = off)
const VALID_INTERVAL_MS = [0, 60_000, 300_000, 900_000, 1_800_000, 3_600_000];

/**
 * Returns true if the current wall-clock time is within NYSE market hours:
 * Monday–Friday, 09:30–16:00 America/New_York.
 *
 * Exported for unit testing.
 */
export function isWithinMarketHours(): boolean {
  const now = new Date();
  const dayOfWeek = now.toLocaleDateString('en-US', {
    timeZone: 'America/New_York',
    weekday: 'long',
  });
  const timeStr = now.toLocaleTimeString('en-US', {
    timeZone: 'America/New_York',
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
  });
  const isWeekday = !['Saturday', 'Sunday'].includes(dayOfWeek);
  const [h, m] = timeStr.split(':').map(Number);
  const minutes = h * 60 + m;
  return isWeekday && minutes >= 9 * 60 + 30 && minutes < 16 * 60;
}

interface UseAutoRefreshOptions {
  onRefresh: () => Promise<void>;
}

interface UseAutoRefreshReturn {
  isEnabled: boolean;
  /** Interval in whole minutes (0 when disabled). */
  intervalMinutes: number;
  marketHoursOnly: boolean;
  /** Seconds remaining until the next auto-refresh. 0 when disabled. */
  secondsUntilRefresh: number;
  /** Set the refresh interval; pass 0 to disable. */
  setInterval: (minutes: number) => void;
  setMarketHoursOnly: (enabled: boolean) => void;
  /** Toggle auto-refresh on/off. */
  toggle: () => void;
  /**
   * Alias for `secondsUntilRefresh | null` — null when disabled.
   * Kept for backward compatibility with TopBar's existing countdown prop.
   */
  countdown: number | null;
}

export function useAutoRefresh({ onRefresh }: UseAutoRefreshOptions): UseAutoRefreshReturn {
  const { value: intervalMsStr, setValue: persistIntervalMs } = useConfig(CONFIG_KEY_INTERVAL, '0');
  const { value: marketHoursStr, setValue: persistMarketHours } = useConfig(
    CONFIG_KEY_MARKET_HOURS,
    'false'
  );

  const intervalMs = VALID_INTERVAL_MS.includes(Number(intervalMsStr)) ? Number(intervalMsStr) : 0;
  const marketHoursOnly = marketHoursStr === 'true';
  const isEnabled = intervalMs > 0;
  const intervalMinutes = isEnabled ? Math.round(intervalMs / 60_000) : 0;

  const [secondsUntilRefresh, setSecondsUntilRefresh] = useState<number>(
    isEnabled ? Math.ceil(intervalMs / 1000) : 0
  );

  // Stable ref so the tick closure always calls the latest onRefresh without
  // restarting the timer every render.
  const onRefreshRef = useRef(onRefresh);
  useEffect(() => {
    onRefreshRef.current = onRefresh;
  }, [onRefresh]);

  // marketHoursOnly ref for the same reason
  const marketHoursOnlyRef = useRef(marketHoursOnly);
  useEffect(() => {
    marketHoursOnlyRef.current = marketHoursOnly;
  }, [marketHoursOnly]);

  const nextRefreshAt = useRef<number | null>(null);
  const timerRef = useRef<ReturnType<typeof globalThis.setInterval> | null>(null);

  useEffect(() => {
    if (timerRef.current !== null) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }

    if (intervalMs <= 0) {
      nextRefreshAt.current = null;
      return;
    }

    nextRefreshAt.current = Date.now() + intervalMs;

    timerRef.current = setInterval(() => {
      // Pause when the tab/window is hidden — reset the fire time so we don't
      // immediately fire when the user returns.
      if (document.hidden) {
        nextRefreshAt.current = Date.now() + intervalMs;
        setSecondsUntilRefresh(Math.ceil(intervalMs / 1000));
        return;
      }

      // Pause outside market hours — reset countdown so it doesn't freeze
      if (marketHoursOnlyRef.current && !isWithinMarketHours()) {
        setSecondsUntilRefresh(Math.ceil(intervalMs / 1000));
        return;
      }

      const remaining = (nextRefreshAt.current ?? 0) - Date.now();
      if (remaining <= 0) {
        nextRefreshAt.current = Date.now() + intervalMs;
        setSecondsUntilRefresh(Math.ceil(intervalMs / 1000));
        void onRefreshRef.current();
      } else {
        setSecondsUntilRefresh(Math.ceil(remaining / 1000));
      }
    }, 1000);

    return () => {
      if (timerRef.current !== null) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [intervalMs]);

  const setIntervalMinutes = useCallback(
    (minutes: number) => {
      void persistIntervalMs(String(minutes * 60_000));
    },
    [persistIntervalMs]
  );

  const setMarketHoursOnly = useCallback(
    (enabled: boolean) => {
      void persistMarketHours(String(enabled));
    },
    [persistMarketHours]
  );

  const toggle = useCallback(() => {
    if (isEnabled) {
      void persistIntervalMs('0');
    } else {
      // Default to 5 minutes when toggling on with no prior interval
      void persistIntervalMs(String(intervalMs > 0 ? intervalMs : 300_000));
    }
  }, [isEnabled, intervalMs, persistIntervalMs]);

  return {
    isEnabled,
    intervalMinutes,
    marketHoursOnly,
    secondsUntilRefresh,
    setInterval: setIntervalMinutes,
    setMarketHoursOnly,
    toggle,
    countdown: isEnabled ? secondsUntilRefresh : null,
  };
}
