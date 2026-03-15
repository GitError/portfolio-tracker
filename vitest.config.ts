import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'happy-dom',
    globals: true,
    setupFiles: ['./src/__tests__/setup.ts'],
    exclude: ['.claude/**', 'node_modules/**'],
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
        'src/__tests__/',
        'src/lib/mockData.ts',
        'src/lib/perfMockData.ts',
      ],
    },
  },
});
