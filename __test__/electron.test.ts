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
const ALLOWED_AUTH = ['granted', 'denied', 'notDetermined', 'unsupported'] as const;

function resolveElectronBinary(): string | null {
  try {
    // electron's main export is the binary path string when loaded from Node.
    return require('electron') as unknown as string;
  } catch {
    return null;
  }
}

const isMacOS = process.platform === 'darwin';
const isWindows = process.platform === 'win32';
const electronBinary = resolveElectronBinary();
const distBuilt = existsSync(DIST_ENTRY);
const guardsPass = electronBinary != null && distBuilt;

const macTest = isMacOS && guardsPass ? test : test.skip;
const winTest = isWindows && guardsPass ? test : test.skip;

describe('macOS — Electron host (dev mode)', () => {
  emitSkipReasons(isMacOS);

  macTest(
    'spawned Electron returns a valid notification status payload',
    async () => {
      const status = await runAndParse('mac', {});
      expect(status.platform).toBe('darwin');
      assertCommonShape(status);
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

describe('Windows — Electron host (dev mode)', () => {
  emitSkipReasons(isWindows);

  // No reason-code assertion — Windows Electron dev-mode behavior is captured
  // for the first time by this run; tighten in a follow-up after observation.
  winTest(
    'spawned Electron without AUMID returns a valid notification status payload',
    async () => {
      const status = await runAndParse('win-no-aumid', {});
      expect(status.platform).toBe('win32');
      assertCommonShape(status);
    },
    30_000,
  );

  winTest(
    'spawned Electron with AUMID set returns a valid notification status payload',
    async () => {
      const status = await runAndParse('win-aumid', {
        NOTIFY_STATUS_AUMID: 'dev.notify-status.electron.fixture',
      });
      expect(status.platform).toBe('win32');
      assertCommonShape(status);
    },
    30_000,
  );
});

function assertCommonShape(status: Record<string, unknown>): void {
  expect(typeof status.authorization).toBe('string');
  expect(typeof status.doNotDisturb).toBe('boolean');
  expect(ALLOWED_AUTH).toContain(status.authorization);
}

async function runAndParse(
  label: string,
  env: Record<string, string>,
): Promise<Record<string, unknown>> {
  const { stdout, stderr, exitCode } = await runFixture(electronBinary as string, env);

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
  console.log(`[electron e2e | ${label}]`, JSON.stringify(status));
  return status;
}

interface FixtureResult {
  stdout: string;
  stderr: string;
  exitCode: number | null;
}

function runFixture(
  electronPath: string,
  extraEnv: Record<string, string>,
): Promise<FixtureResult> {
  return new Promise((resolve, reject) => {
    const child = spawn(electronPath, [FIXTURE_DIR], {
      cwd: path.resolve(__dirname, '..'),
      stdio: ['ignore', 'pipe', 'pipe'],
      env: { ...process.env, ...extraEnv },
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

function emitSkipReasons(isTargetPlatform: boolean): void {
  if (!isTargetPlatform) return;
  if (electronBinary == null) {
    test.skip('electron not installed — install devDependency to enable', () => {});
  }
  if (!distBuilt) {
    test.skip(`dist/index.cjs missing — run \`pnpm run build:ts\` first (expected at ${DIST_ENTRY})`, () => {});
  }
}
