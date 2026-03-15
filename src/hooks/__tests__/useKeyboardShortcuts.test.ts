import { renderHook } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { createElement } from 'react';
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

const wrapper = ({ children }: { children: React.ReactNode }) =>
  createElement(MemoryRouter, null, children);

describe('useKeyboardShortcuts', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('calls onRefresh when metaKey+r is pressed', () => {
    const onRefresh = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onRefresh }), { wrapper });

    fireKeydown({ key: 'r', metaKey: true });

    expect(onRefresh).toHaveBeenCalledOnce();
  });

  it('calls onRefresh when ctrlKey+r is pressed', () => {
    const onRefresh = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onRefresh }), { wrapper });

    fireKeydown({ key: 'r', ctrlKey: true });

    expect(onRefresh).toHaveBeenCalledOnce();
  });

  it('does NOT call onRefresh when target is an INPUT element', () => {
    const onRefresh = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onRefresh }), { wrapper });

    const inputEl = document.createElement('input');
    fireKeydown({ key: 'r', metaKey: true, target: inputEl });

    expect(onRefresh).not.toHaveBeenCalled();
  });

  it('does NOT call onRefresh when target is a TEXTAREA element', () => {
    const onRefresh = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onRefresh }), { wrapper });

    const textarea = document.createElement('textarea');
    fireKeydown({ key: 'r', metaKey: true, target: textarea });

    expect(onRefresh).not.toHaveBeenCalled();
  });

  it('does NOT call onRefresh when target is a SELECT element', () => {
    const onRefresh = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onRefresh }), { wrapper });

    const select = document.createElement('select');
    fireKeydown({ key: 'r', metaKey: true, target: select });

    expect(onRefresh).not.toHaveBeenCalled();
  });

  it('does NOT call onRefresh when target is contenteditable', () => {
    const onRefresh = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onRefresh }), { wrapper });

    const div = document.createElement('div');
    div.contentEditable = 'true';
    fireKeydown({ key: 'r', metaKey: true, target: div });

    expect(onRefresh).not.toHaveBeenCalled();
  });

  it('navigates to "/" when metaKey+1 is pressed', () => {
    renderHook(() => useKeyboardShortcuts({}), { wrapper });
    // No throw = navigation was attempted (navigate is internal to the hook)
    expect(() => fireKeydown({ key: '1', metaKey: true })).not.toThrow();
  });

  it('navigates to "/holdings" when metaKey+2 is pressed', () => {
    renderHook(() => useKeyboardShortcuts({}), { wrapper });
    expect(() => fireKeydown({ key: '2', metaKey: true })).not.toThrow();
  });

  it('navigates to "/performance" when metaKey+3 is pressed', () => {
    renderHook(() => useKeyboardShortcuts({}), { wrapper });
    expect(() => fireKeydown({ key: '3', metaKey: true })).not.toThrow();
  });

  it('navigates to "/stress" when metaKey+4 is pressed', () => {
    renderHook(() => useKeyboardShortcuts({}), { wrapper });
    expect(() => fireKeydown({ key: '4', metaKey: true })).not.toThrow();
  });

  it('navigates to "/settings" when metaKey+, is pressed', () => {
    renderHook(() => useKeyboardShortcuts({}), { wrapper });
    expect(() => fireKeydown({ key: ',', metaKey: true })).not.toThrow();
  });

  it('calls onToggleHelp when ? key is pressed without modifier', () => {
    const onToggleHelp = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onToggleHelp }), { wrapper });

    fireKeydown({ key: '?' });

    expect(onToggleHelp).toHaveBeenCalledOnce();
  });

  it('does NOT call onToggleHelp when ? is pressed with metaKey', () => {
    const onToggleHelp = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onToggleHelp }), { wrapper });

    fireKeydown({ key: '?', metaKey: true });

    expect(onToggleHelp).not.toHaveBeenCalled();
  });

  it('calls onOpenAddHolding when metaKey+n is pressed', () => {
    const onOpenAddHolding = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onOpenAddHolding }), { wrapper });

    fireKeydown({ key: 'n', metaKey: true });

    expect(onOpenAddHolding).toHaveBeenCalledOnce();
  });

  it('does NOT call onOpenAddHolding when target is an INPUT element', () => {
    const onOpenAddHolding = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onOpenAddHolding }), { wrapper });

    const inputEl = document.createElement('input');
    fireKeydown({ key: 'n', metaKey: true, target: inputEl });

    expect(onOpenAddHolding).not.toHaveBeenCalled();
  });

  it('calls onExportCsv when metaKey+e is pressed', () => {
    const onExportCsv = vi.fn();
    renderHook(() => useKeyboardShortcuts({ onExportCsv }), { wrapper });

    fireKeydown({ key: 'e', metaKey: true });

    expect(onExportCsv).toHaveBeenCalledOnce();
  });

  it('removes the event listener on unmount', () => {
    const onRefresh = vi.fn();
    const removeEventListenerSpy = vi.spyOn(window, 'removeEventListener');

    const { unmount } = renderHook(() => useKeyboardShortcuts({ onRefresh }), { wrapper });
    unmount();

    expect(removeEventListenerSpy).toHaveBeenCalledWith('keydown', expect.any(Function));
  });

  it('does not throw when callbacks are undefined', () => {
    expect(() => {
      renderHook(() => useKeyboardShortcuts({}), { wrapper });
      fireKeydown({ key: 'r', metaKey: true });
      fireKeydown({ key: '?' });
      fireKeydown({ key: '1', metaKey: true });
      fireKeydown({ key: ',', metaKey: true });
    }).not.toThrow();
  });
});
