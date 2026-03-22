import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'happy-dom',
    globals: true,
    setupFiles: ['./frontend/__tests__/setup.ts'],
    exclude: ['.claude/**', 'node_modules/**', 'e2e/**'],
    server: {
      deps: {
        // Force ESM-only packages through Vite's transform pipeline
        inline: [/@csstools/, /@asamuzakjp/],
      },
    },
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/',
        'frontend/__tests__/',
        'frontend/lib/mockData.ts',
        'frontend/lib/perfMockData.ts',
      ],
    },
  },
});
