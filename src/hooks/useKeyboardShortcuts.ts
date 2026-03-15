import { useEffect } from 'react';

export interface KeyboardShortcutCallbacks {
  onRefresh?: () => void;
  onAddHolding?: () => void;
  onNavigate?: (path: string) => void;
  onToggleHelp?: () => void;
}

const NAVIGATE_KEYS: Record<string, string> = {
  '1': '/',
  '2': '/holdings',
  '3': '/performance',
  '4': '/stress',
};

function isEditableTarget(target: EventTarget | null): boolean {
  if (!target) return false;
  const el = target as HTMLElement;
  const tag = el.tagName;
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
  if (el.isContentEditable) return true;
  return false;
}

export function useKeyboardShortcuts(callbacks: KeyboardShortcutCallbacks): void {
  const { onRefresh, onAddHolding, onNavigate, onToggleHelp } = callbacks;

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent): void {
      const isMeta = e.metaKey || e.ctrlKey;

      if (isEditableTarget(e.target)) return;

      // Cmd/Ctrl+R — refresh prices
      if (isMeta && e.key === 'r') {
        e.preventDefault();
        onRefresh?.();
        return;
      }

      // Cmd/Ctrl+N — add holding
      if (isMeta && e.key === 'n') {
        e.preventDefault();
        onAddHolding?.();
        return;
      }

      // Cmd/Ctrl+1..4 — navigate
      if (isMeta && NAVIGATE_KEYS[e.key]) {
        onNavigate?.(NAVIGATE_KEYS[e.key]);
        return;
      }

      // ? (no modifier) — toggle help
      if (!isMeta && e.key === '?') {
        onToggleHelp?.();
        return;
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onRefresh, onAddHolding, onNavigate, onToggleHelp]);
}
