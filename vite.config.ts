import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    environment: 'node',
    include: ['__test__/**/*.test.ts'],
    exclude: ['**/node_modules/**', '**/dist/**', '**/target/**', '**/*.test-d.ts'],
    testTimeout: 60_000,
  },
  pack: {
    entry: ['src/index.ts'],
    dts: true,
    format: ['esm', 'cjs'],
    sourcemap: true,
    deps: {
      neverBundle: [/binding\.(js|cjs|mjs)$/],
    },
  },
  lint: {
    ignorePatterns: [
      'dist/**',
      'target/**',
      'node_modules/**',
      'binding.cjs',
      'binding.d.cts',
      'notify-status.*.node',
      // tsd files import from ../dist/index.mjs which only exists after `vp pack`.
      // tsd has its own type-check pass via `vp run test:types`.
      '__test__/**/*.test-d.ts',
    ],
    options: {
      typeAware: true,
      typeCheck: true,
    },
  },
  fmt: {
    ignorePatterns: [
      'dist/**',
      'target/**',
      'node_modules/**',
      'binding.cjs',
      'binding.d.cts',
      'notify-status.*.node',
      'docs/**',
      'CHANGELOG.md',
      'README.md',
      'Cargo.toml',
      'Cargo.lock',
      'rust-toolchain.toml',
      'rustfmt.toml',
      'src/**/*.rs',
    ],
    singleQuote: true,
    trailingComma: 'all',
  },
});
