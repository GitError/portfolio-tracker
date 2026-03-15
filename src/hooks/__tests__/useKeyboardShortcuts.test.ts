import { renderHook } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useKeyboardShortcuts } from '../useKeyboardShortcuts';

function fireKeydown(overrides: Partial<KeyboardEventInit> & { target?: EventTarget }): void {
  const { target, ...init } = overrides;
  const event = new KeyboardEvent('keydown', { bubbles: true, ...init });
  if (target) {
    Object.defineProperty(event, 'target', { value: target, configurable: true });
  }
  window.dispatchEvent(event);
}

describe('useKeyboardShortcuts', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('calls onRefresh when metaKey+r is pressed', () => {
    const onRefresh = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onRefresh }));

    fireKeydown({ key: 'r', metaKey: true });

    expect(onRefresh).toHaveBeenCalledOnce();
  });

  it('calls onRefresh when ctrlKey+r is pressed', () => {
    const onRefresh = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onRefresh }));

    fireKeydown({ key: 'r', ctrlKey: true });

    expect(onRefresh).toHaveBeenCalledOnce();
  });

  it('does NOT call onRefresh when target is an INPUT element', () => {
    const onRefresh = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onRefresh }));

    const inputEl = document.createElement('input');
    fireKeydown({ key: 'r', metaKey: true, target: inputEl });

    expect(onRefresh).not.toHaveBeenCalled();
  });

  it('does NOT call onRefresh when target is a TEXTAREA element', () => {
    const onRefresh = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onRefresh }));

    const textarea = document.createElement('textarea');
    fireKeydown({ key: 'r', metaKey: true, target: textarea });

    expect(onRefresh).not.toHaveBeenCalled();
  });

  it('does NOT call onRefresh when target is a SELECT element', () => {
    const onRefresh = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onRefresh }));

    const select = document.createElement('select');
    fireKeydown({ key: 'r', metaKey: true, target: select });

    expect(onRefresh).not.toHaveBeenCalled();
  });

  it('does NOT call onRefresh when target is contenteditable', () => {
    const onRefresh = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onRefresh }));

    const div = document.createElement('div');
    div.contentEditable = 'true';
    fireKeydown({ key: 'r', metaKey: true, target: div });

    expect(onRefresh).not.toHaveBeenCalled();
  });

  it('calls onNavigate("/") when metaKey+1 is pressed', () => {
    const onNavigate = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onNavigate }));

    fireKeydown({ key: '1', metaKey: true });

    expect(onNavigate).toHaveBeenCalledWith('/');
  });

  it('calls onNavigate("/holdings") when metaKey+2 is pressed', () => {
    const onNavigate = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onNavigate }));

    fireKeydown({ key: '2', metaKey: true });

    expect(onNavigate).toHaveBeenCalledWith('/holdings');
  });

  it('calls onNavigate("/performance") when metaKey+3 is pressed', () => {
    const onNavigate = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onNavigate }));

    fireKeydown({ key: '3', metaKey: true });

    expect(onNavigate).toHaveBeenCalledWith('/performance');
  });

  it('calls onNavigate("/stress") when metaKey+4 is pressed', () => {
    const onNavigate = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onNavigate }));

    fireKeydown({ key: '4', metaKey: true });

    expect(onNavigate).toHaveBeenCalledWith('/stress');
  });

  it('calls onToggleHelp when ? key is pressed without modifier', () => {
    const onToggleHelp = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onToggleHelp }));

    fireKeydown({ key: '?' });

    expect(onToggleHelp).toHaveBeenCalledOnce();
  });

  it('does NOT call onToggleHelp when ? is pressed with metaKey', () => {
    const onToggleHelp = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onToggleHelp }));

    // ? with metaKey should not trigger help
    fireKeydown({ key: '?', metaKey: true });

    expect(onToggleHelp).not.toHaveBeenCalled();
  });

  it('calls onAddHolding when metaKey+n is pressed', () => {
    const onAddHolding = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onAddHolding }));

    fireKeydown({ key: 'n', metaKey: true });

    expect(onAddHolding).toHaveBeenCalledOnce();
  });

  it('does NOT call onAddHolding when target is an INPUT element', () => {
    const onAddHolding = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onAddHolding }));

    const inputEl = document.createElement('input');
    fireKeydown({ key: 'n', metaKey: true, target: inputEl });

    expect(onAddHolding).not.toHaveBeenCalled();
  });

  it('removes the event listener on unmount', () => {
    const onRefresh = vi.fn();
    const removeEventListenerSpy = vi.spyOn(window, 'removeEventListener');

    const { unmount } = renderHook(() => useKeyboardShortcuts({ onRefresh }));
    unmount();

    expect(removeEventListenerSpy).toHaveBeenCalledWith('keydown', expect.any(Function));
  });

  it('does not throw when callbacks are undefined', () => {
    expect(() => {
      renderHook(() => useKeyboardShortcuts({}));
      fireKeydown({ key: 'r', metaKey: true });
      fireKeydown({ key: '?' });
      fireKeydown({ key: '1', metaKey: true });
    }).not.toThrow();
  });
});
