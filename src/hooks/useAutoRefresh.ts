import { useEffect, useRef, useState } from 'react';

interface UseAutoRefreshOptions {
  intervalMs: number; // 0 = disabled
  onRefresh: () => Promise<void>;
  pauseWhenHidden?: boolean;
}

interface UseAutoRefreshReturn {
  /** Seconds until next refresh, or null when disabled. */
  countdown: number | null;
}

export function useAutoRefresh({
  intervalMs,
  onRefresh,
  pauseWhenHidden = true,
}: UseAutoRefreshOptions): UseAutoRefreshReturn {
  const [countdown, setCountdown] = useState<number | null>(null);
  const nextRefreshAt = useRef<number | null>(null);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    if (intervalMs <= 0) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setCountdown(null);
      nextRefreshAt.current = null;
      if (timerRef.current) clearInterval(timerRef.current);
      return;
    }

    nextRefreshAt.current = Date.now() + intervalMs;

    timerRef.current = setInterval(() => {
      if (pauseWhenHidden && document.hidden) {
        // Shift the next fire time so we don't accumulate debt
        nextRefreshAt.current = Date.now() + intervalMs;
        setCountdown(Math.ceil(intervalMs / 1000));
        return;
      }

      const remaining = (nextRefreshAt.current ?? 0) - Date.now();
      if (remaining <= 0) {
        nextRefreshAt.current = Date.now() + intervalMs;
        setCountdown(Math.ceil(intervalMs / 1000));
        void onRefresh();
      } else {
        setCountdown(Math.ceil(remaining / 1000));
      }
    }, 1000);

    // Initial countdown
    setCountdown(Math.ceil(intervalMs / 1000));

    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [intervalMs, onRefresh, pauseWhenHidden]);

  return { countdown };
}
