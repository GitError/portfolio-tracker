// Tauri v1 exposed `__TAURI__` on `window`; Tauri v2 exposes `__TAURI_INTERNALS__`.
// We support both so the app uses the real SQLite-backed commands in either runtime.
export const isTauri = (): boolean =>
  typeof window !== 'undefined' && ('__TAURI__' in window || '__TAURI_INTERNALS__' in window);

export async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}
