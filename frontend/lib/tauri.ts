// Tauri v1 exposed `__TAURI__` on `window`; Tauri v2 exposes `__TAURI_INTERNALS__`.
// We support both so the app uses the real SQLite-backed commands in either runtime.
export const isTauri = (): boolean =>
  typeof window !== 'undefined' && ('__TAURI__' in window || '__TAURI_INTERNALS__' in window);

/**
 * Extracts a human-readable message from an unknown thrown/rejected value.
 * Tauri IPC rejections are plain strings or plain `{ type, message }` objects
 * (see AppError's serde representation), never `Error` instances.
 */
export function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === 'string') {
    return error;
  }
  if (
    error &&
    typeof error === 'object' &&
    'message' in error &&
    typeof (error as { message: unknown }).message === 'string'
  ) {
    return (error as { message: string }).message;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

export async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw new Error(getErrorMessage(e));
  }
}
