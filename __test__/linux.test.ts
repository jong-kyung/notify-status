/// <reference types="node" />

import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, test } from "vite-plus/test";

import { getNotificationStatus } from "../src/index.js";

// Exercise package exports with no binding.cjs or .node artifacts. Platform
// overrides stay inside child processes and do not simulate native OS APIs.
describe.each(["import", "require"])("%s without native artifacts", (format) => {
  test.each(["linux", "freebsd", "darwin", "win32"])("%s", (platform) => {
    const directory = mkdtempSync(join(tmpdir(), "notify-status-fallback-"));
    try {
      mkdirSync(join(directory, "dist"));
      copyFileSync(new URL("../package.json", import.meta.url), join(directory, "package.json"));
      const entry = `index.${format === "import" ? "mjs" : "cjs"}`;
      copyFileSync(new URL(`../dist/${entry}`, import.meta.url), join(directory, "dist", entry));
      const child = spawnSync(
        process.execPath,
        [
          "--input-type=module",
          "-e",
          `
            import assert from 'node:assert/strict';
            import { createRequire } from 'node:module';
            Object.defineProperty(process, 'platform', { value: '${platform}' });
            const load = () => ${format === "import" ? "import('notify-status')" : "createRequire(import.meta.url)('notify-status')"};
            if (${platform === "darwin" || platform === "win32"}) {
              await assert.rejects(async () => load(), { code: 'MODULE_NOT_FOUND' });
            } else {
              const { getNotificationStatus, isEffectivelyEnabled } = await load();
              const results = await Promise.all(Array.from({ length: 10 }, () => getNotificationStatus()));
              for (const result of results) {
                assert.deepEqual(result, {
                  authorization: 'unsupported',
                  doNotDisturb: false,
                  platform: '${platform}',
                  reason: 'unsupportedPlatform',
                });
                assert.equal(isEffectivelyEnabled(result), false);
              }
            }
          `,
        ],
        { cwd: directory, encoding: "utf8", timeout: 10_000 },
      );
      expect(child.error).toBeUndefined();
      expect(child.status, child.stderr).toBe(0);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
});

const isLinux = process.platform === "linux";
const linuxTest = isLinux ? test : test.skip;

// Covers AE5 / R8 — Linux is intentionally not supported. The library must
// return `{ authorization: 'unsupported', reason: 'unsupportedPlatform' }`
// and never throw. This guarantees safe import in Electron apps that ship
// a Linux build even though the feature is a no-op there.
describe("Linux — AE5 (unsupported platform)", () => {
  linuxTest("returns unsupported(unsupportedPlatform) without crashing", async () => {
    const status = await getNotificationStatus();
    expect(status.platform).toBe("linux");
    expect(status.authorization).toBe("unsupported");
    expect(status.doNotDisturb).toBe(false);
    expect(status.reason).toBe("unsupportedPlatform");
  });

  linuxTest("the function never throws", async () => {
    await expect(getNotificationStatus()).resolves.toBeDefined();
  });

  linuxTest("ten concurrent calls all resolve identically", async () => {
    const results = await Promise.all(Array.from({ length: 10 }, () => getNotificationStatus()));
    for (const r of results) {
      expect(r.platform).toBe("linux");
      expect(r.authorization).toBe("unsupported");
      expect(r.reason).toBe("unsupportedPlatform");
    }
  });
});
