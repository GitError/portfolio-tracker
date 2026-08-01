import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';

describe('getErrorMessage', () => {
  afterEach(() => {
    vi.resetModules();
  });

  it('returns the message from a real Error instance', async () => {
    const { getErrorMessage } = await import('../tauri');
    expect(getErrorMessage(new Error('boom'))).toBe('boom');
  });

  it('returns a plain string rejection unchanged', async () => {
    const { getErrorMessage } = await import('../tauri');
    expect(getErrorMessage('database is locked')).toBe('database is locked');
  });

  it('extracts message from a Tauri AppError-shaped object', async () => {
    const { getErrorMessage } = await import('../tauri');
    expect(
      getErrorMessage({ type: 'Validation', message: 'quantity must be a positive finite number' })
    ).toBe('quantity must be a positive finite number');
  });

  it('falls back to JSON.stringify for shapes without a message field', async () => {
    const { getErrorMessage } = await import('../tauri');
    expect(getErrorMessage({ code: 42 })).toBe('{"code":42}');
  });
});

describe('tauriInvoke error normalization', () => {
  beforeEach(() => {
    vi.resetModules();
  });

  afterEach(() => {
    vi.resetModules();
    vi.doUnmock('@tauri-apps/api/core');
  });

  it('rejects with a real Error instance carrying the backend message when the command rejects with a plain object', async () => {
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke: vi.fn().mockRejectedValue({ type: 'NotFound', message: 'holding not found' }),
    }));

    const { tauriInvoke } = await import('../tauri');

    await expect(tauriInvoke('get_portfolio')).rejects.toBeInstanceOf(Error);
    await expect(tauriInvoke('get_portfolio')).rejects.toThrow('holding not found');
  });

  it('rejects with a real Error instance when the command rejects with a plain string', async () => {
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke: vi.fn().mockRejectedValue('quantity must be a positive finite number'),
    }));

    const { tauriInvoke } = await import('../tauri');

    await expect(tauriInvoke('add_holding')).rejects.toBeInstanceOf(Error);
    await expect(tauriInvoke('add_holding')).rejects.toThrow(
      'quantity must be a positive finite number'
    );
  });

  it('resolves normally when the command succeeds', async () => {
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke: vi.fn().mockResolvedValue({ totalValue: 100 }),
    }));

    const { tauriInvoke } = await import('../tauri');

    await expect(tauriInvoke('get_portfolio')).resolves.toEqual({ totalValue: 100 });
  });
});
