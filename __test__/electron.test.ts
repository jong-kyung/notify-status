import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

import { describe, expect, test } from 'vite-plus/test';

const require = createRequire(import.meta.url);

const FIXTURE_DIR = path.resolve(__dirname, 'electron-fixture');
const DIST_ENTRY = path.resolve(__dirname, '..', 'dist', 'index.cjs');
const SENTINEL_BEGIN = '===NOTIFY_STATUS_BEGIN===';
const SENTINEL_END = '===NOTIFY_STATUS_END===';

function resolveElectronBinary(): string | null {
  try {
    // electron's main export is the binary path string when loaded from Node.
    return require('electron') as unknown as string;
  } catch {
    return null;
  }
}

const isMacOS = process.platform === 'darwin';
const electronBinary = resolveElectronBinary();
const distBuilt = existsSync(DIST_ENTRY);

const electronTest = isMacOS && electronBinary != null && distBuilt ? test : test.skip;

describe('macOS — Electron host (dev mode)', () => {
  if (isMacOS && electronBinary == null) {
    test.skip('electron not installed — install devDependency to enable', () => {});
    return;
  }
  if (isMacOS && !distBuilt) {
    test.skip(`dist/index.cjs missing — run \`pnpm run build:ts\` first (expected at ${DIST_ENTRY})`, () => {});
    return;
  }

  electronTest(
    'spawned Electron returns a valid notification status payload',
    async () => {
      const { stdout, stderr, exitCode } = await runFixture(electronBinary as string);

      expect(
        exitCode,
        `electron exited with non-zero code.\nstdout:\n${stdout}\nstderr:\n${stderr}`,
      ).toBe(0);

      const beginIdx = stdout.indexOf(SENTINEL_BEGIN);
      const endIdx = stdout.indexOf(SENTINEL_END);
      expect(
        beginIdx,
        `BEGIN sentinel not found in stdout.\nstdout:\n${stdout}\nstderr:\n${stderr}`,
      ).toBeGreaterThanOrEqual(0);
      expect(
        endIdx,
        `END sentinel not found in stdout.\nstdout:\n${stdout}\nstderr:\n${stderr}`,
      ).toBeGreaterThan(beginIdx);

      const jsonText = stdout.slice(beginIdx + SENTINEL_BEGIN.length, endIdx).trim();
      let parsed: unknown;
      try {
        parsed = JSON.parse(jsonText);
      } catch (err) {
        throw new Error(
          `Failed to parse fixture output as JSON.\nraw: ${JSON.stringify(jsonText)}\nerror: ${String(err)}`,
        );
      }

      expect(typeof parsed).toBe('object');
      expect(parsed).not.toBeNull();
      const status = parsed as Record<string, unknown>;

      // eslint-disable-next-line no-console
      console.log('[electron e2e] captured status:', JSON.stringify(status));

      expect(status.platform).toBe('darwin');
      expect(typeof status.authorization).toBe('string');
      expect(typeof status.doNotDisturb).toBe('boolean');
      // authorization value is non-deterministic on CI/dev — assert shape only.
      expect(['granted', 'denied', 'notDetermined', 'unsupported']).toContain(status.authorization);

      expect(
        status.reason,
        `Electron host should not produce noBundleId — the host bundle is com.github.Electron. Got: ${JSON.stringify(status)}`,
      ).not.toBe('noBundleId');
      expect(
        status.reason,
        `Unexpected internalError from Electron host. Captured: ${JSON.stringify(status)}`,
      ).not.toBe('internalError');
    },
    30_000,
  );
});

interface FixtureResult {
  stdout: string;
  stderr: string;
  exitCode: number | null;
}

function runFixture(electronPath: string): Promise<FixtureResult> {
  return new Promise((resolve, reject) => {
    const child = spawn(electronPath, [FIXTURE_DIR], {
      cwd: path.resolve(__dirname, '..'),
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk: Buffer) => {
      stdout += chunk.toString('utf8');
    });
    child.stderr.on('data', (chunk: Buffer) => {
      stderr += chunk.toString('utf8');
    });

    child.on('error', (err) => reject(err));
    child.on('close', (exitCode) => resolve({ stdout, stderr, exitCode }));
  });
}
