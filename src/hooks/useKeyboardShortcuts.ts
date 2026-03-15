import { useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';

const ROUTE_KEYS: Record<string, string> = {
  '1': '/',
  '2': '/holdings',
  '3': '/performance',
  '4': '/stress',
};

function isInFormField(target: EventTarget | null): boolean {
  if (!target) return false;
  const el = target as HTMLElement;
  const tag = el.tagName;
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
  // Also check contenteditable
  if (el.isContentEditable) return true;
  return false;
}

export interface KeyboardShortcutsOptions {
  onRefresh?: () => void;
  onOpenAddHolding?: () => void;
  onExportCsv?: () => void;
  onToggleHelp?: () => void;
  /** @deprecated use onOpenAddHolding */
  onAddHolding?: () => void;
  /** @deprecated use individual navigate shortcuts */
  onNavigate?: (path: string) => void;
}

export function useKeyboardShortcuts({
  onRefresh,
  onOpenAddHolding,
  onExportCsv,
  onToggleHelp,
}: KeyboardShortcutsOptions): void {
  const navigate = useNavigate();
  const location = useLocation();

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (isInFormField(e.target)) return;

      const isMeta = e.metaKey || e.ctrlKey;

      // ⌘R / Ctrl+R — refresh prices
      if (isMeta && e.key === 'r') {
        e.preventDefault();
        onRefresh?.();
        return;
      }

      // ⌘N / Ctrl+N — open Add Holding modal
      if (isMeta && e.key === 'n') {
        e.preventDefault();
        onOpenAddHolding?.();
        return;
      }

      // ⌘E / Ctrl+E — export CSV (only relevant on holdings page, but fires globally)
      if (isMeta && e.key === 'e') {
        e.preventDefault();
        onExportCsv?.();
        return;
      }

      // ⌘1–4 / Ctrl+1–4 — navigate between views
      if (isMeta && ROUTE_KEYS[e.key]) {
        e.preventDefault();
        navigate(ROUTE_KEYS[e.key]);
        return;
      }

      // ? — toggle keyboard shortcuts help overlay (no modifier)
      if (!isMeta && e.key === '?') {
        e.preventDefault();
        onToggleHelp?.();
        return;
      }

      // 1–4 — navigate views without modifier
      if (!isMeta && ROUTE_KEYS[e.key]) {
        navigate(ROUTE_KEYS[e.key]);
        return;
      }
    }

    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [navigate, location, onRefresh, onOpenAddHolding, onExportCsv, onToggleHelp]);
}
