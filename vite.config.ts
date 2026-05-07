import { defineConfig } from 'vite-plus';

const IGNORE_PATTERNS_BASE = [
  'dist/**',
  'target/**',
  'node_modules/**',
  'binding.cjs',
  'binding.d.cts',
  'notify-status.*.node',
];

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
    ignorePatterns: [...IGNORE_PATTERNS_BASE, '**/*.test-d.ts'],
    options: {
      typeAware: true,
      typeCheck: true,
    },
  },
  fmt: {
    ignorePatterns: [
      ...IGNORE_PATTERNS_BASE,
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
