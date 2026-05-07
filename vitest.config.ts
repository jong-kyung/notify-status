import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node',
    include: ['__test__/**/*.test.ts'],
    exclude: [
      '**/node_modules/**',
      '**/dist/**',
      '**/target/**',
      '**/*.test-d.ts',
    ],
    testTimeout: 60_000,
  },
});
