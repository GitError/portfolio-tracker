import path from 'path';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// @ts-expect-error process is a nodejs global
const isProd = process.env.NODE_ENV === 'production';

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],

  resolve: {
    alias: isProd
      ? {
          // In production builds, replace mock data modules with empty stubs so they are
          // excluded from the bundle. isTauri() is always true at runtime in Tauri, making
          // the mock branches dead code that would otherwise inflate bundle size.
          './mockData': path.resolve(__dirname, 'frontend/lib/mockData.prod.ts'),
          '../lib/mockData': path.resolve(__dirname, 'frontend/lib/mockData.prod.ts'),
          './perfMockData': path.resolve(__dirname, 'frontend/lib/perfMockData.prod.ts'),
          '../lib/perfMockData': path.resolve(__dirname, 'frontend/lib/perfMockData.prod.ts'),
        }
      : {},
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  build: { sourcemap: 'hidden' },
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
}));
